use std::path::Path;

use swc_atoms::JsWord;
use swc_ecma_ast::{
  BindingIdent, ClassDecl, Decl, DefaultDecl, EmptyStmt, EsVersion, Expr, FnDecl, Ident,
  ModuleDecl, ModuleItem, Pat, Stmt, VarDecl, VarDeclarator,
};

use swc_common::sync::Lrc;
use swc_common::{
  errors::{ColorConfig, Handler},
  FileName, SourceMap,
};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};
use swc_ecma_parser::{EsConfig, TsConfig};

mod hook;
mod statement;
pub use hook::*;
pub use statement::*;

pub mod path {
  pub fn relative_id(id: String) -> String {
    if nodejs_path::is_absolute(&id) {
      nodejs_path::relative(&nodejs_path::resolve!("."), &id)
    } else {
      id
    }
  }
}

pub fn is_external_module(source: &str) -> bool {
  !nodejs_path::is_absolute(source) && !source.starts_with(".")
}

pub fn parse_file(
  source_code: String,
  filename: &str,
  src_map: &Lrc<SourceMap>,
) -> Result<swc_ecma_ast::Module, swc_ecma_parser::error::Error> {
  let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(src_map.clone()));
  let p = Path::new(filename);
  let fm = src_map.new_source_file(FileName::Custom(filename.to_owned()), source_code);
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

pub fn parse_code(code: &str) -> Result<swc_ecma_ast::Module, ()> {
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

use swc_common::DUMMY_SP;

pub fn fold_export_decl_to_decl(
  module_item: &mut ModuleItem,
  default_name: &JsWord,
  is_entry: bool,
) {
  if let ModuleItem::ModuleDecl(module_decl) = &module_item {
    *module_item = match module_decl {
      // remove export { foo, baz }
      // FIXME: this will also remove `export * as foo from './foo'`. How we handle this?
      ModuleDecl::ExportNamed(_) => ModuleItem::Stmt(Stmt::Empty(EmptyStmt { span: DUMMY_SP })),
      // remove `export` from `export class Foo {...}`
      ModuleDecl::ExportDecl(export_decl) => ModuleItem::Stmt(Stmt::Decl(export_decl.decl.clone())),
      // remove `export default` from `export default class Foo {...}`
      ModuleDecl::ExportDefaultDecl(export_decl) => {
        if let DefaultDecl::Class(node) = &export_decl.decl {
          ModuleItem::Stmt(Stmt::Decl(Decl::Class(ClassDecl {
            ident: node
              .ident
              .clone()
              .unwrap_or_else(|| Ident::new(default_name.clone(), DUMMY_SP)),
            declare: false,
            class: node.class.clone(),
          })))
        } else if let DefaultDecl::Fn(node) = &export_decl.decl {
          ModuleItem::Stmt(Stmt::Decl(Decl::Fn(FnDecl {
            ident: node
              .ident
              .clone()
              .unwrap_or_else(|| Ident::new(default_name.clone(), DUMMY_SP)),
            declare: false,
            function: node.clone().function,
          })))
        } else {
          ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(export_decl.clone()))
        }
      }
      ModuleDecl::ExportDefaultExpr(export_decl) => {
        // `export () => {}` => `const _default = () => {}`
        if let Expr::Arrow(node) = export_decl.expr.as_ref() {
          ModuleItem::Stmt(Stmt::Decl(Decl::Var(VarDecl {
            span: DUMMY_SP,
            kind: swc_ecma_ast::VarDeclKind::Var,
            declare: false,
            decls: vec![VarDeclarator {
              span: DUMMY_SP,
              name: Pat::Ident(BindingIdent {
                id: Ident::new(default_name.clone(), DUMMY_SP),
                type_ann: None,
              }),
              definite: false,
              init: Some(Box::new(Expr::Arrow(node.clone()))),
            }],
          })))
        } else {
          create_empty_statement()
        }
      }
      ModuleDecl::ExportAll(export_all) => {
        // keep external module as it (we may use it later on code-gen) and internal modules removed.
        if is_entry && is_external_module(export_all.src.value.as_ref()) {
          ModuleItem::ModuleDecl(module_decl.clone())
        } else {
          create_empty_statement()
        }
      }
      _ => create_empty_statement(),
    };
  }
}
