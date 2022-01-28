use crate::ast;
use crate::plugin_driver::PluginDriver;
use crate::symbol_box::SymbolBox;
use crate::utils::{ast_sugar, resolve_id};
use std::collections::HashMap;

use std::sync::{Arc, Mutex};
use std::{collections::HashSet, hash::Hash};

use ast::{
  BindingIdent, ClassDecl, Decl, DefaultDecl, Expr, FnDecl, ModuleDecl, ModuleItem, Pat, Stmt,
  VarDecl, VarDeclarator,
};
use swc_atoms::JsWord;

use swc_common::util::take::Take;
use swc_common::{Mark, SyntaxContext, DUMMY_SP};
use swc_ecma_ast::Ident;

use swc_ecma_visit::{noop_visit_mut_type, VisitMut};

use crate::scanner::rel::{ExportDesc, ReExportDesc};
use crate::types::{IsExternal, ResolvedId};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Namespace {
  pub included: bool,
  pub mark: Mark,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Module {
  pub ast: ast::Module,
  pub id: String,
  pub local_exports: HashMap<JsWord, ExportDesc>,
  pub re_exports: HashMap<JsWord, ReExportDesc>,
  pub re_export_all_sources: HashSet<JsWord>,
  pub exports: HashMap<JsWord, Mark>,
  pub declared_symbols: HashMap<JsWord, Mark>,
  pub imported_symbols: HashMap<JsWord, Mark>,
  pub resolved_ids: HashMap<JsWord, ResolvedId>,
  pub suggested_names: HashMap<JsWord, JsWord>,
  pub namespace: Namespace,
  pub is_user_defined_entry_point: bool,
}

impl Module {
  pub fn new(id: String) -> Self {
    Self {
      ast: ast::Module::dummy(),
      id,
      local_exports: Default::default(),
      re_export_all_sources: Default::default(),
      re_exports: Default::default(),
      exports: Default::default(),
      resolved_ids: Default::default(),
      suggested_names: Default::default(),
      declared_symbols: Default::default(),
      imported_symbols: Default::default(),
      namespace: Default::default(),
      is_user_defined_entry_point: false,
    }
  }

  pub fn link_local_exports(&mut self) {
    self.local_exports.iter().for_each(|(key, info)| {
      log::debug!("curr id: {} key: {}", self.id, key);
      self.exports.insert(key.clone(), info.mark);
    });
    self.re_exports.iter().for_each(|(key, info)| {
      self.exports.insert(key.clone(), info.mark);
    });
    // We couldn't deal with `export * from './foo'` now.
  }

  pub fn bind_local_references(&self, symbol_box: &mut SymbolBox) {
    self
      .local_exports
      .iter()
      .for_each(|(_exported_name, export_desc)| {
        let name = if let Some(default_exported_ident) = &export_desc.identifier {
          default_exported_ident
        } else {
          // we need local_name. For `export { foo as bar }`, we need `foo` to bind references.
          &export_desc.local_name
        };
        if name == "default" {
          // This means that the module's `export default` is a value. No name to bind.
          // And we need to generate a name for it lately.
          return;
        }
        let symbol_mark = self.resolve_mark(name);
        symbol_box.union(export_desc.mark, symbol_mark);
      });
  }

  pub fn include(&mut self) {
    self
      .ast
      .body
      .retain(|module_item| !matches!(module_item, ModuleItem::ModuleDecl(ModuleDecl::Import(_))));
  }

  pub fn suggest_name(&mut self, name: JsWord, suggested: JsWord) {
    self.suggested_names.insert(name, suggested);
  }

  pub fn resolve_id(
    &mut self,
    dep_src: &JsWord,
    plugin_driver: &Mutex<PluginDriver>,
    external: Arc<Mutex<Vec<IsExternal>>>,
  ) -> ResolvedId {
    self
      .resolved_ids
      .entry(dep_src.clone())
      .or_insert_with_key(|key| {
        resolve_id(
          key,
          Some(&self.id),
          false,
          &plugin_driver.lock().unwrap(),
          external,
        )
      })
      .clone()
  }

  pub fn resolve_mark(&self, name: &JsWord) -> Mark {
    *self.declared_symbols.get(name).unwrap_or_else(|| {
      self
        .imported_symbols
        .get(name)
        .expect(&format!("unkown name: {:?} for module {}", name, self.id))
    })
  }

  pub fn trim_exports(&mut self) {
    let body = self.ast.body.take();
    self.ast.body = body
      .into_iter()
      .map(|module_item| fold_export_decl_to_decl(module_item, self))
      .collect();
  }

  pub fn generate_exports(&mut self) {
    if !self.exports.is_empty() {
      let export_decl = ast_sugar::export(&self.exports);
      self.ast.body.push(ModuleItem::ModuleDecl(export_decl));
    }
  }

  pub fn include_namespace(&mut self) {
    if !self.namespace.included {
      let suggested_default_export_name = self
        .suggested_names
        .get(&"*".into())
        .map(|s| s.clone())
        .unwrap_or_else(|| {
          (get_valid_name(nodejs_path::parse(&self.id).name) + "namespace").into()
        });
      // TODO: we might need to check if the name already exsits.
      assert!(!self
        .declared_symbols
        .contains_key(&suggested_default_export_name));
      self.local_exports.insert(
        "*".to_string().into(),
        ExportDesc {
          identifier: None,
          mark: self.namespace.mark,
          local_name: suggested_default_export_name.clone(),
        },
      );
      self
        .exports
        .insert("*".to_string().into(), self.namespace.mark);
      let namespace = ast_sugar::namespace(
        (suggested_default_export_name.clone(), self.namespace.mark),
        &self
          .exports
          .iter()
          .filter(|(exported_name, _mark)| *exported_name != "*")
          .map(|(exported_name, mark)| (exported_name.clone(), *mark))
          .collect::<Vec<_>>(),
      );
      self
        .declared_symbols
        .insert(suggested_default_export_name, self.namespace.mark);
      self.ast.body.push(ModuleItem::Stmt(namespace));
      // self.ast.body.push();
      self.namespace.included = true;
    }
  }
}

impl std::fmt::Debug for Module {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Module")
      .field("id", &self.id)
      .field("local_exports", &self.local_exports)
      .field("re_exports", &self.re_exports)
      .field("re_export_all_sources", &self.re_export_all_sources)
      .field("exports", &self.exports)
      .field("declared_symbols", &self.declared_symbols)
      .field("imported_symbols", &self.imported_symbols)
      .field("resolved_ids", &self.resolved_ids)
      .field("suggested_names", &self.suggested_names)
      .finish()
  }
}

impl Hash for Module {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    state.write(&self.id.as_bytes());
  }
}

#[derive(Clone, Copy)]
struct ClearMark;
impl VisitMut for ClearMark {
  noop_visit_mut_type!();

  fn visit_mut_ident(&mut self, ident: &mut Ident) {
    ident.span.ctxt = SyntaxContext::empty();
  }
}

// FIXME: Not robost
fn get_valid_name(name: String) -> String {
  name.chars().filter(|c| c != &'.').collect()
}

pub fn fold_export_decl_to_decl(
  module_item: ModuleItem,
  module: &mut Module,
  // is_entry: bool,
) -> ModuleItem {
  let mut get_default_ident = || {
    let suggested_default_export_name = module
      .suggested_names
      .get(&"default".into())
      .map(|s| s.clone())
      .unwrap_or_else(|| get_valid_name(nodejs_path::parse(&module.id).name).into());

    assert!(!module
      .declared_symbols
      .contains_key(&suggested_default_export_name));
    module.declared_symbols.insert(
      suggested_default_export_name.clone(),
      module.exports.get(&"default".into()).unwrap().clone(),
    );

    Ident::new(suggested_default_export_name, DUMMY_SP)
  };
  if let ModuleItem::ModuleDecl(module_decl) = module_item {
    match module_decl {
      // remove `export` from `export class Foo {...}`
      ModuleDecl::ExportDecl(export_decl) => ModuleItem::Stmt(Stmt::Decl(export_decl.decl)),

      // remove `export default` from `export default class Foo {...}` or `export default class {...}`
      ModuleDecl::ExportDefaultDecl(export_decl) => match export_decl.decl {
        DefaultDecl::Class(node) => ModuleItem::Stmt(Stmt::Decl(Decl::Class(ClassDecl {
          ident: node.ident.unwrap_or_else(get_default_ident),
          declare: false,
          class: node.class,
        }))),
        DefaultDecl::Fn(node) => ModuleItem::Stmt(Stmt::Decl(Decl::Fn(FnDecl {
          ident: node.ident.unwrap_or_else(get_default_ident),
          declare: false,
          function: node.function,
        }))),
        _ => ModuleItem::dummy(),
      },
      ModuleDecl::ExportAll(export_all) => {
        // keep external module as it (we may use it later on code-gen) and internal modules removed.
        // export * from 'react'
        if module
          .resolved_ids
          .get(&export_all.src.value)
          .unwrap()
          .external
          .unwrap_or_default()
        {
          ModuleItem::ModuleDecl(ModuleDecl::ExportAll(export_all))
        } else {
          // remove `export * from './foo'`
          ModuleItem::dummy()
        }
      }
      ModuleDecl::ExportDefaultExpr(export_decl) => {
        // ignore `export default foo`
        if let Expr::Ident(_) = export_decl.expr.as_ref() {
          ModuleItem::dummy()
        } else {
          // change `export () => {}` => `const _default = () => {}`
          ModuleItem::Stmt(Stmt::Decl(Decl::Var(VarDecl {
            span: DUMMY_SP,
            kind: swc_ecma_ast::VarDeclKind::Var,
            declare: false,
            decls: vec![VarDeclarator {
              span: DUMMY_SP,
              name: Pat::Ident(BindingIdent {
                id: get_default_ident(),
                type_ann: None,
              }),
              definite: false,
              init: Some(export_decl.expr.clone()),
            }],
          })))
        }
      }
      // remove `export { foo, baz }`
      _ => ModuleItem::dummy(),
    }
  } else {
    module_item
  }
}
