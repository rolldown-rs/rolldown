use std::collections::HashSet;

use swc_atoms::JsWord;
use swc_common::Mark;
use swc_ecma_ast::{
  CallExpr, Callee, Decl, DefaultDecl, ExportSpecifier, Expr, Lit, ModuleDecl, ModuleExportName,
};

use crate::{ext::SyntaxContextExt, graph::Rel};

use super::{helper::collect_js_word_of_pat, Scanner};

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct ExportDesc {
  // export foo; foo is identifier;
  pub identifier: Option<JsWord>,
  pub local_name: JsWord,
  pub mark: Mark,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct ReExportDesc {
  // name in importee
  pub original: JsWord,
  // locally defined name
  pub local_name: JsWord,
  pub source: JsWord,
  pub mark: Mark,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct DynImportDesc {
  pub argument: JsWord,
  pub id: Option<JsWord>,
}

impl Scanner {
  pub fn add_import(&mut self, module_decl: &ModuleDecl) {
    if let ModuleDecl::Import(import_decl) = module_decl {
      let source = &import_decl.src.value;
      let import_info = self
        .import_infos
        .entry(source.clone())
        .or_insert_with(|| {
          let rel = RelationInfo::new(source.clone(), self.cur_relation_order);
          self.cur_relation_order += 1;
          rel
        });

      // We separate each specifier to support later tree-shaking.
      import_decl.specifiers.iter().for_each(|specifier| {
        let used;
        let original;
        let mark;
        match specifier {
          // import foo from './foo'
          swc_ecma_ast::ImportSpecifier::Default(n) => {
            used = n.local.sym.clone();
            original = "default".into();
            mark = n.local.span.ctxt.as_mark();
          }
          // import { foo } from './foo'
          // import { foo as foo2 } from './foo'
          swc_ecma_ast::ImportSpecifier::Named(n) => {
            used = n.local.sym.clone();
            original = n
              .imported // => foo2 in `import { foo as foo2 } from './foo'`
              .as_ref()
              .map_or(used.clone(), |module_export_name| {
                if let ModuleExportName::Ident(ident) = module_export_name {
                  ident.sym.clone()
                } else {
                  panic!("")
                }
              });
            mark = n.local.span.ctxt.as_mark();
          }
          // import * as foo from './foo'
          swc_ecma_ast::ImportSpecifier::Namespace(n) => {
            used = n.local.sym.clone();
            original = "*".into();
            mark = n.local.span.ctxt.as_mark();
          }
        }
        import_info.names.insert(Specifier {
          original,
          used,
          mark,
        });
      });
    }
  }

  pub fn add_dynamic_import(&mut self, call_exp: &CallExpr) {
    if let Callee::Import(_import) = &call_exp.callee {
      // FIXME: should warn about pattern like `import(...a)`
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

  pub fn add_export(&mut self, module_decl: &ModuleDecl) {
    match module_decl {
      ModuleDecl::ExportDefaultDecl(node) => {
        let identifier = match &node.decl {
          DefaultDecl::Class(node) => node.ident.as_ref().map(|id| id.sym.clone()),
          DefaultDecl::Fn(node) => node.ident.as_ref().map(|id| id.sym.clone()),
          _ => None,
        };
        // TODO: what's the meaning of Mark for default export
        self.local_exports.insert(
          "default".into(),
          ExportDesc {
            identifier,
            local_name: "default".into(),
            mark: self.symbol_box.lock().unwrap().new_mark(),
          },
        );
      }
      ModuleDecl::ExportDefaultExpr(node) => {
        // export default foo;
        let identifier: Option<JsWord> = match node.expr.as_ref() {
          Expr::Ident(id) => Some(id.sym.clone()),
          _ => None,
        };
        self.local_exports.insert(
          "default".into(),
          ExportDesc {
            identifier,
            local_name: "default".into(),
            mark: self.symbol_box.lock().unwrap().new_mark(),
          },
        );
      }
      ModuleDecl::ExportNamed(node) => {
        node.specifiers.iter().for_each(|specifier| {
          match specifier {
            ExportSpecifier::Named(s) => {
              if let Some(source_node) = &node.src {
                let source = source_node.value.clone();
                let re_export_info =
                  self
                    .re_export_infos
                    .entry(source.clone())
                    .or_insert_with(|| {
                      let rel = RelationInfo {
                        source: source.clone(),
                        names: Default::default(),
                        order: self.cur_relation_order,
                      };
                      self.cur_relation_order += 1;
                      rel
                    });
                // export { name } from './other'
                let source = source_node.value.clone();
                let name = s
                  .exported
                  .as_ref()
                  .map_or(get_sym_from_module_export(&s.orig).clone(), |id| {
                    get_sym_from_module_export(id)
                  });
                let re_export_mark = self.symbol_box.lock().unwrap().new_mark();
                re_export_info.names.insert(Specifier {
                  original: get_sym_from_module_export(&s.orig).clone(),
                  used: name.clone(),
                  mark: re_export_mark,
                });
                self.re_exports.insert(
                  name.clone(),
                  ReExportDesc {
                    local_name: get_sym_from_module_export(&s.orig).clone(),
                    source,
                    original: name.clone(),
                    mark: re_export_mark,
                  },
                );
              } else {
                // export { foo, bar, baz }
                log::debug!("export var {:#?}", s);
                let local_name = get_sym_from_module_export(&s.orig).clone();
                let exported_name: JsWord = s
                  .exported
                  .as_ref()
                  .map_or(get_sym_from_module_export(&s.orig), |id| {
                    get_sym_from_module_export(&id).clone()
                  });
                self.local_exports.insert(
                  exported_name.clone(),
                  ExportDesc {
                    identifier: None,
                    local_name,
                    mark: self.symbol_box.lock().unwrap().new_mark(),
                  },
                );
              };
            }
            ExportSpecifier::Namespace(s) => {
              let source = node.src.as_ref().map(|str| str.value.clone()).unwrap();
              let re_export_info =
                self
                  .re_export_infos
                  .entry(source.clone())
                  .or_insert_with(|| {
                    let rel = RelationInfo {
                      source: source.clone(),
                      names: Default::default(),
                      order: self.cur_relation_order,
                    };
                    self.cur_relation_order += 1;
                    rel
                  });
              let re_export_mark = self.symbol_box.lock().unwrap().new_mark();

              re_export_info.names.insert(Specifier {
                original: "*".into(),
                used: get_sym_from_module_export(&s.name).clone(),
                mark: re_export_mark,
              });
              // export * as name from './other'
              let name = get_sym_from_module_export(&s.name).clone();
              self.re_exports.insert(
                name.clone(),
                ReExportDesc {
                  local_name: "*".into(),
                  source,
                  original: name.clone(),
                  mark: re_export_mark,
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
            self.local_exports.insert(
              local_name.clone(),
              ExportDesc {
                identifier: None,
                local_name,
                mark: self.symbol_box.lock().unwrap().new_mark(),
              },
            );
          }
          Decl::Fn(node) => {
            // export function foo () {}
            let local_name = node.ident.sym.clone();
            self.local_exports.insert(
              local_name.clone(),
              ExportDesc {
                identifier: None,
                local_name,
                mark: self.symbol_box.lock().unwrap().new_mark(),
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
                  self.local_exports.insert(
                    local_name.clone(),
                    ExportDesc {
                      identifier: None,
                      local_name,
                      mark: self.symbol_box.lock().unwrap().new_mark(),
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
        self.export_all_sources.insert((node.src.value.clone(), self.cur_relation_order));
        self.cur_relation_order += 1;
      }
      _ => {}
    }
  }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Specifier {
  /// The original defined name 
  pub original: JsWord,
  /// The name importer used
  pub used: JsWord,
  pub mark: Mark,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RelationInfo {
  pub source: JsWord,
  // Empty HashSet represents `import './side-effect'` or `import {} from './foo'`
  pub names: HashSet<Specifier>,
  pub order: usize,
}

impl From<RelationInfo> for Rel {
  fn from(info: RelationInfo) -> Self {
    Self::Import(info)
  }
}

impl RelationInfo {
  pub fn new(source: JsWord, order: usize) -> Self {
    Self {
      source,
      names: Default::default(),
      order,
      // namespace: Default::default(),
    }
  }
}

#[inline]
fn get_sym_from_module_export(module_export_name: &ModuleExportName) -> JsWord {
  match module_export_name {
    ModuleExportName::Ident(i) => i.sym.clone(),
    _ => panic!(""),
  }
}
