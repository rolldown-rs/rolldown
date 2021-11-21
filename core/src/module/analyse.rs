use std::collections::HashSet;
use std::{collections::HashMap, path::Path, sync::Arc};

use swc_common::DUMMY_SP;
use swc_ecma_ast::{
  BindingIdent, ClassDecl, Decl, DefaultDecl, EmptyStmt, EsVersion, ExportSpecifier, Expr, FnDecl,
  Ident, ImportDecl, ModuleDecl, ModuleItem, Pat, Stmt, VarDecl, VarDeclarator,
};

use crate::ast;

use super::Module;
use swc_common::sync::Lrc;
use swc_common::{
  errors::{ColorConfig, Handler},
  FileName, SourceMap,
};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};
use swc_ecma_parser::{EsConfig, TsConfig};

fn add_import(
  module_decl: &ModuleDecl,
  imports: &mut HashMap<String, ImportDesc>,
  module_id: &str,
) {
  if let ModuleDecl::Import(import_decl) = module_decl {
    import_decl.specifiers.iter().for_each(|specifier| {
      let local_name;
      let name;
      match specifier {
        // import foo from './foo'
        swc_ecma_ast::ImportSpecifier::Default(n) => {
          local_name = n.local.sym.to_string();
          name = "default".to_owned();
        }
        // import { foo } from './foo'
        // import { foo as foo2 } from './foo'
        swc_ecma_ast::ImportSpecifier::Named(n) => {
          local_name = n.local.sym.to_string();
          name = n
            .imported
            .as_ref()
            .map_or(local_name.clone(), |ident| ident.sym.to_string());
        }
        // import * as foo from './foo'
        swc_ecma_ast::ImportSpecifier::Namespace(n) => {
          local_name = n.local.sym.to_string();
          name = "*".to_owned()
        }
      }
      imports.insert(
        local_name.clone(),
        ImportDesc {
          module_id: module_id.into(),
          source: import_decl.src.value.to_string(),
          name,
          local_name,
        },
      );
    })
  }
}

fn add_export(
  module_decl: &ModuleDecl,
  exports: &mut HashMap<String, ExportDesc>,
  re_exports: &mut HashMap<String, ReExportDesc>,
  export_all_sources: &mut HashSet<String>,
  module_id: &str,
) {
  match module_decl {
    ModuleDecl::ExportDefaultDecl(node) => {
      let identifier = match &node.decl {
        DefaultDecl::Class(node) => node.ident.as_ref().map(|id| id.sym.to_string()),
        DefaultDecl::Fn(node) => node.ident.as_ref().map(|id| id.sym.to_string()),
        _ => None,
      };

      exports.insert(
        "default".into(),
        ExportDesc {
          identifier,
          local_name: "default".to_owned(),
        },
      );
    }
    ModuleDecl::ExportDefaultExpr(node) => {
      // export default foo;
      let identifier = match node.expr.as_ref() {
        Expr::Ident(id) => Some(id.sym.to_string()),
        _ => None,
      };
      exports.insert(
        "default".into(),
        ExportDesc {
          identifier,
          local_name: "default".into(),
        },
      );
    }
    ModuleDecl::ExportNamed(node) => {
      node.specifiers.iter().for_each(|specifier| {
        match specifier {
          ExportSpecifier::Named(s) => {
            if let Some(source_node) = &node.src {
              // export { name } from './other'
              let source = source_node.value.to_string();
              let name = s
                .exported
                .as_ref()
                .map_or(s.orig.sym.to_string(), |id| id.sym.to_string());
              re_exports.insert(
                name.clone(),
                ReExportDesc {
                  local_name: s.orig.sym.to_string(),
                  module_id: module_id.into(),
                  source,
                },
              );
            } else {
              // export { foo, bar, baz }
              let local_name = s.orig.sym.to_string();
              let exported_name = s
                .exported
                .as_ref()
                .map_or(s.orig.sym.to_string(), |id| id.sym.to_string());
              exports.insert(
                exported_name,
                ExportDesc {
                  identifier: None,
                  local_name,
                },
              );
            };
          }
          ExportSpecifier::Namespace(s) => {
            // export * as name from './other'
            let source = node.src.as_ref().map(|str| str.value.to_string()).unwrap();
            let name = s.name.sym.to_string();
            re_exports.insert(
              name.clone(),
              ReExportDesc {
                local_name: "*".into(),
                module_id: module_id.into(),
                source,
              },
            );
          }
          ExportSpecifier::Default(_) => {
            // export v from 'mod';
            // Rollup doesn't support it.
          }
        };
      })
    }
    ModuleDecl::ExportDecl(node) => {
      match &node.decl {
        Decl::Class(node) => {
          // export class Foo {}
          let local_name = node.ident.sym.to_string();
          exports.insert(
            local_name.clone(),
            ExportDesc {
              identifier: None,
              local_name,
            },
          );
        }
        Decl::Fn(node) => {
          // export function foo () {}
          let local_name = node.ident.sym.to_string();
          exports.insert(
            local_name.clone(),
            ExportDesc {
              identifier: None,
              local_name,
            },
          );
        }
        Decl::Var(node) => {
          // export var { foo, bar } = ...
          // export var foo = 1, bar = 2;
          node.decls.iter().for_each(|decl| {
            ast::helper::collect_names_of_pat(&decl.name)
              .into_iter()
              .for_each(|local_name| {
                exports.insert(
                  local_name.clone(),
                  ExportDesc {
                    identifier: None,
                    local_name,
                  },
                );
              });
          });
        }
        _ => {}
      }
    }
    ModuleDecl::ExportAll(node) => {
      // export * from './other'
      export_all_sources.insert(node.src.value.to_string());
    }
    _ => {}
  }
}

impl Module {
  pub fn analyse(
    body: &[ModuleItem],
    module_id: &str,
  ) -> (
    HashMap<String, ImportDesc>,
    HashMap<String, ExportDesc>,
    HashMap<String, ReExportDesc>,
    HashSet<String>,
  ) {
    let mut imports = HashMap::new();
    let mut exports = HashMap::new();
    let mut re_exports = HashMap::new();
    let mut export_all_sources = HashSet::new();

    body
      .iter()
      .flat_map(|module_item| {
        if let ModuleItem::ModuleDecl(module_decl) = module_item {
          Some(module_decl)
        } else {
          None
        }
      })
      .for_each(|module_decl| {
        add_import(module_decl, &mut imports, module_id);
        add_export(
          module_decl,
          &mut exports,
          &mut re_exports,
          &mut export_all_sources,
          module_id,
        );
      });

    (imports, exports, re_exports, export_all_sources)
  }
}

#[derive(Clone)]
pub struct ImportDesc {
  pub module_id: String,
  pub source: String,
  pub name: String,
  pub local_name: String,
}

pub struct ExportDesc {
  pub identifier: Option<String>,
  pub local_name: String,
}

pub struct ReExportDesc {
  pub module_id: String,
  pub local_name: String,
  pub source: String,
}

pub fn parse_file(
  source_code: String,
  filename: String,
  src_map: &Lrc<SourceMap>,
) -> Result<swc_ecma_ast::Module, swc_ecma_parser::error::Error> {
  let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(src_map.clone()));
  let p = Path::new(filename.as_str());
  let fm = src_map.new_source_file(FileName::Custom(filename.clone()), source_code);
  let ext = p.extension().and_then(|ext| ext.to_str()).unwrap_or("js");
  let syntax = if ext == "ts" || ext == "tsx" {
    Syntax::Typescript(TsConfig {
      dynamic_import: true,
      decorators: false,
      import_assertions: true,
      tsx: ext == "tsx",
      ..Default::default()
    })
  } else {
    Syntax::Es(EsConfig {
      dynamic_import: true,
      num_sep: true,
      static_blocks: true,
      private_in_object: true,
      import_assertions: true,
      top_level_await: true,
      import_meta: true,
      jsx: ext == "jsx",
      optional_chaining: true,
      nullish_coalescing: true,
      export_namespace_from: true,
      export_default_from: true,
      decorators_before_export: true,
      decorators: true,
      fn_bind: true,
      class_props: true,
      class_private_props: true,
      class_private_methods: true,
    })
  };

  let lexer = Lexer::new(
    syntax,
    EsVersion::latest(),
    StringInput::from(fm.as_ref()),
    None,
  );

  let mut parser = Parser::new_from(lexer);

  parser.take_errors().into_iter().for_each(|e| {
    e.into_diagnostic(&handler).emit();
  });
  parser.parse_module()
}

#[cfg(test)]
pub(crate) fn parse_code(code: &str) -> Result<swc_ecma_ast::Module, ()> {
  use swc_common::BytePos;
  let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, None);
  let lexer = Lexer::new(
    // We want to parse ecmascript
    Syntax::Es(EsConfig::default()),
    // JscTarget defaults to es5
    EsVersion::latest(),
    StringInput::new(code, BytePos(0), BytePos(0)),
    None,
  );

  let mut parser = Parser::new_from(lexer);

  parser.take_errors().into_iter().for_each(|e| {
    e.into_diagnostic(&handler).emit();
  });
  parser.parse_module().map_err(|e| {
    // Unrecoverable fatal error occurred
    e.into_diagnostic(&handler).emit()
  })
}
