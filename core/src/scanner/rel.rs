use swc_atoms::JsWord;
use swc_common::{Mark, SyntaxContext};
use swc_ecma_ast::{
  CallExpr, Decl, DefaultDecl, ExportSpecifier, Expr, ExprOrSuper, Lit, ModuleDecl,
};

use super::{helper::collect_js_word_of_pat, Scanner};

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct ImportDesc {
  pub source: JsWord,
  // name in importer
  pub name: JsWord,
  // orignal defined name
  pub local_name: JsWord,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct ExportDesc {
  pub identifier: Option<JsWord>,
  pub local_name: JsWord,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct ReExportDesc {
  // name in importee
  pub name: JsWord,
  // locally defined name
  pub local_name: JsWord,
  pub source: JsWord,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct DynImportDesc {
  pub argument: JsWord,
  pub id: Option<JsWord>,
}

impl Scanner {
  pub fn add_import(&mut self, module_decl: &ModuleDecl) {
    if let ModuleDecl::Import(import_decl) = module_decl {
      let source = import_decl.src.value.clone();
      self.sources.insert(source);
      if import_decl.specifiers.len() > 0 {
        import_decl.specifiers.iter().for_each(|specifier| {
          let local_name;
          let name;
          match specifier {
            // import foo from './foo'
            swc_ecma_ast::ImportSpecifier::Default(n) => {
              local_name = n.local.sym.clone();
              name = "default".into();
            }
            // import { foo } from './foo'
            // import { foo as foo2 } from './foo'
            swc_ecma_ast::ImportSpecifier::Named(n) => {
              local_name = n.local.sym.clone();
              name = n
                .imported // => foo2 in `import { foo as foo2 } from './foo'`
                .as_ref()
                .map_or(local_name.clone(), |ident| ident.sym.clone());
            }
            // import * as foo from './foo'
            swc_ecma_ast::ImportSpecifier::Namespace(n) => {
              local_name = n.local.sym.clone();
              name = "*".into()
            }
          }
          self.imports.insert(
            local_name.clone(),
            ImportDesc {
              // module: module.clone().into(),
              source: import_decl.src.value.clone(),
              name,
              local_name,
            },
          );
        })
      } else {
        // FIXME: we should handle case like `import './side-effect'` or 'import {} from './foo''
      }
    }
  }

  pub fn add_dynamic_import(&mut self, call_exp: &CallExpr) {
    if let ExprOrSuper::Expr(exp) = &call_exp.callee {
      if let Expr::Ident(id) = exp.as_ref() {
        let is_callee_import = id.sym.to_string() == "import";
        // FIXME: should warn about pattern like `import(...a)`
        if is_callee_import {
          if let Some(exp) = call_exp
            .args
            .get(0)
            .map(|exp_or_spread| &exp_or_spread.expr)
          {
            if let Expr::Lit(Lit::Str(first_param)) = exp.as_ref() {
              self.dynamic_imports.insert(DynImportDesc {
                argument: first_param.value.clone(),
                id: None,
              });
            } else {
              panic!("unkown dynamic import params")
            }
          }
        }
      }
    }
  }

  pub fn add_export(
    &mut self,
    module_decl: &ModuleDecl,
    // exports: &mut HashMap<JsWord, ExportDesc>,
    // re_exports: &mut HashMap<JsWord, ReExportDesc>,
    // export_all_sources: &mut HashSet<JsWord>,
    // sources: &mut HashSet<JsWord>,
  ) {
    match module_decl {
      ModuleDecl::ExportDefaultDecl(node) => {
        let identifier = match &node.decl {
          DefaultDecl::Class(node) => node.ident.as_ref().map(|id| id.sym.clone()),
          DefaultDecl::Fn(node) => node.ident.as_ref().map(|id| id.sym.clone()),
          _ => None,
        };

        self.exports.insert(
          "default".into(),
          ExportDesc {
            identifier,
            local_name: "default".into(),
          },
        );
      }
      ModuleDecl::ExportDefaultExpr(node) => {
        // export default foo;
        let identifier: Option<JsWord> = match node.expr.as_ref() {
          Expr::Ident(id) => Some(id.sym.clone()),
          _ => None,
        };
        self.exports.insert(
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
                let source = source_node.value.clone();
                self.sources.insert(source.clone());
                let name = s
                  .exported
                  .as_ref()
                  .map_or(s.orig.sym.clone(), |id| id.sym.clone());
                self.re_exports.insert(
                  name.clone(),
                  ReExportDesc {
                    local_name: s.orig.sym.clone(),
                    source,
                    name: name.clone(),
                  },
                );
              } else {
                // export { foo, bar, baz }
                let local_name = s.orig.sym.clone();
                let exported_name: JsWord = s
                  .exported
                  .as_ref()
                  .map_or(s.orig.sym.clone(), |id| id.sym.clone());
                self.exports.insert(
                  exported_name.clone(),
                  ExportDesc {
                    identifier: None,
                    local_name,
                  },
                );
              };
            }
            ExportSpecifier::Namespace(s) => {
              // export * as name from './other'
              let source = node.src.as_ref().map(|str| str.value.clone()).unwrap();
              self.sources.insert(source.clone());
              let name = s.name.sym.clone();
              self.re_exports.insert(
                name.clone(),
                ReExportDesc {
                  local_name: "*".into(),
                  source,
                  name: name.clone(),
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
            let local_name = node.ident.sym.clone();
            self.exports.insert(
              local_name.clone(),
              ExportDesc {
                identifier: None,
                local_name,
              },
            );
          }
          Decl::Fn(node) => {
            // export function foo () {}
            let local_name = node.ident.sym.clone();
            self.exports.insert(
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
              collect_js_word_of_pat(&decl.name)
                .into_iter()
                .for_each(|local_name| {
                  self.exports.insert(
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
        self.sources.insert(node.src.value.clone());
        self.export_all_sources.insert(node.src.value.clone());
      }
      _ => {}
    }
  }
}
