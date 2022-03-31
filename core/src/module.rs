use crate::ast;
use crate::scanner::ModuleItemInfo;
use crate::statement::Statement;
use crate::symbol_box::SymbolBox;

use crate::utils::{ast_sugar, resolve_id};
use dashmap::DashMap;
use rayon::prelude::*;
use std::collections::HashMap;

use std::sync::Arc;
use std::{collections::HashSet, hash::Hash};

use ast::{
  BindingIdent, ClassDecl, Decl, DefaultDecl, EmptyStmt, Expr, FnDecl, ModuleDecl, ModuleItem, Pat,
  Stmt, VarDecl, VarDeclarator,
};
use smol_str::SmolStr;
use swc_atoms::JsWord;

use swc_common::util::take::Take;
use swc_common::{Mark, Span, SyntaxContext, DUMMY_SP};
use swc_ecma_ast::Ident;

use crate::utils::is_decl_or_stmt;
use swc_ecma_codegen::text_writer::WriteJs;
use swc_ecma_codegen::Emitter;
use swc_ecma_visit::{noop_visit_mut_type, VisitMut};

use crate::scanner::rel::{ExportDesc, ReExportDesc};
use crate::types::ResolvedId;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Namespace {
  pub included: bool,
  pub mark: Mark,
}

#[derive(Clone)]
pub struct Module {
  // resolved_ids is using for caching.
  pub resolved_ids: DashMap<JsWord, ResolvedId>,
  pub statements: Vec<Statement>,
  pub definitions: HashMap<JsWord, usize>,
  pub id: SmolStr,
  pub local_exports: HashMap<JsWord, ExportDesc>,
  pub re_exports: HashMap<JsWord, ReExportDesc>,
  pub re_export_all_sources: HashSet<JsWord>,
  pub exports: HashMap<JsWord, Mark>,
  pub declared_symbols: HashMap<JsWord, Mark>,
  pub imported_symbols: HashMap<JsWord, Mark>,
  pub suggested_names: HashMap<JsWord, JsWord>,
  pub namespace: Namespace,
  pub is_user_defined_entry_point: bool,
  pub module_span: Span,
  // pub module_item_infos: Vec<ModuleItemInfo>,
}

impl Module {
  pub fn new(id: SmolStr) -> Self {
    Self {
      definitions: Default::default(),
      statements: Default::default(),
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
      module_span: Take::dummy(),
    }
  }

  pub fn link_local_exports(&mut self) {
    self.local_exports.iter().for_each(|(key, info)| {
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
        let refernenced_name = export_desc
          .identifier
          .as_ref()
          .unwrap_or(&export_desc.local_name);
        if refernenced_name == "default" {
          // This means that the module's `export default` is a value. Sush as `export default 1`
          // No name to bind. And we need to generate a name for it lately.
          return;
        }
        let symbol_mark = self.resolve_mark(refernenced_name);
        symbol_box.union(export_desc.mark, symbol_mark);
      });
  }

  pub fn set_statements(
    &mut self,
    ast: ast::Module,
    module_item_infos: Vec<ModuleItemInfo>,
    mark_to_stmt: Arc<DashMap<Mark, (SmolStr, usize)>>,
  ) {
    self.module_span = ast.span;
    self.statements = ast
      .body
      .into_iter()
      .zip(module_item_infos.into_iter())
      .enumerate()
      .map(|(idx, (node, info))| {
        let is_decl_or_stmt = is_decl_or_stmt(&node);
        let mut stmt = Statement::new(node);
        if let Some(export_mark) = info.export_mark {
          mark_to_stmt
            .entry(export_mark)
            .or_insert_with(|| (self.id.clone(), idx));
        }
        info.declared.iter().for_each(|(name, mark)| {
          self.definitions.insert(name.clone(), idx);

          // Skip declarations brought by `import`
          if is_decl_or_stmt {
            mark_to_stmt
              .entry(*mark)
              .or_insert_with(|| (self.id.clone(), idx));
          }
        });
        stmt.writes = info.writes;
        stmt.reads = info.reads;
        stmt.side_effect = info.side_effect;
        // TODO: add it back later
        // if stmt.side_effect.is_none() {
        //   let has_unknown_name = stmt
        //     .reads
        //     .iter()
        //     .chain(stmt.writes.iter())
        //     .any(|name| !self.declared_symbols.contains_key(name));
        //   if has_unknown_name {
        //     // TODO: Should do this in Scanner
        //     stmt.side_effect = Some(SideEffect::VisitGlobalVar)
        //   }
        // }

        stmt
      })
      .collect();
  }

  pub fn include_mark(&mut self, name: &JsWord, mark: &Mark) {
    log::debug!(
      "[treeshake]: definition `{}` included in `{}`",
      &name,
      self.id
    );
    if let Some(&stmt_idx) = self.definitions.get(name) {
      let stmt = &mut self.statements[stmt_idx];
      stmt.reads.insert(*mark);
    }
  }

  pub fn include(&mut self, only_side_effects: bool) {
    if only_side_effects {
      self
        .statements
        .par_iter_mut()
        .filter(|stmt| stmt.side_effect.is_some())
        .for_each(|stmt| {
          stmt.include();
        });
    } else {
      self.statements.par_iter_mut().for_each(|stmt| {
        stmt.include();
      });
    }
  }

  pub fn suggest_name(&mut self, name: JsWord, suggested: JsWord) {
    self.suggested_names.insert(name, suggested);
  }

  pub fn resolve_id(&self, dep_src: &JsWord) -> ResolvedId {
    self
      .resolved_ids
      .entry(dep_src.clone())
      .or_insert_with(|| resolve_id(dep_src, Some(&self.id), false))
      .clone()
  }

  pub fn resolve_mark(&self, name: &JsWord) -> Mark {
    *self.declared_symbols.get(name).unwrap_or_else(|| {
      self
        .imported_symbols
        .get(name)
        // TODO: how can we support global exports? such as `export { Math }`
        .unwrap_or_else(|| panic!("unkown name: {:?} for module {}", name, self.id))
    })
  }

  pub fn trim_exports(&mut self) {
    self.statements = self
      .statements
      .take()
      .into_iter()
      .map(|mut stmt| {
        stmt.node = fold_export_decl_to_decl(stmt.node.take(), self);
        stmt
      })
      .collect();
  }

  pub fn generate_exports(&mut self) {
    if !self.exports.is_empty() {
      let export_decl = ast_sugar::export(&self.exports);
      let mut s = Statement::new(ModuleItem::ModuleDecl(export_decl));
      s.include();
      self.statements.push(s);
    }
  }

  pub fn include_namespace(&mut self, mark_to_stmt: Arc<DashMap<Mark, (SmolStr, usize)>>) {
    if !self.namespace.included {
      let suggested_default_export_name = self
        .suggested_names
        .get(&"*".into())
        .cloned()
        .unwrap_or_else(|| {
          (get_valid_name(nodejs_path::parse(&self.id).name) + "namespace").into()
        });
      // TODO: We should generate a name which has no conflict.
      // TODO: We might need to check if the name already exsits.
      assert!(!self
        .declared_symbols
        .contains_key(&suggested_default_export_name));
      self.local_exports.insert(
        "*".into(),
        ExportDesc {
          identifier: None,
          mark: self.namespace.mark,
          local_name: suggested_default_export_name.clone(),
        },
      );
      self.exports.insert("*".into(), self.namespace.mark);
      let namespace = ast_sugar::namespace(
        (suggested_default_export_name.clone(), self.namespace.mark),
        &self.exports,
      );
      let mut s = Statement::new(ast::ModuleItem::Stmt(namespace));
      let idx = self.statements.len();
      self
        .definitions
        .insert(suggested_default_export_name.clone(), idx);
      s.declared
        .entry(suggested_default_export_name.clone())
        .or_insert_with(|| self.namespace.mark);

      mark_to_stmt
        .entry(self.namespace.mark)
        .or_insert_with(|| (self.id.clone(), idx));
      // FIXME: actually we should determine whether we should push a new helper statement by analyse importStar usage
      // eg: import * as foo from "./foo"
      // console.log(foo) // this will include the helper statement
      // console.log(foo.bar) // instead this will only include statement bar itself, and `foo.bar` or `foo['bar']` will be replaced to the defined statement
      // console.log(foo, foo.bar) // this will include the helper statement

      self.statements.push(s);
      self
        .declared_symbols
        .insert(suggested_default_export_name, self.namespace.mark);
      self.namespace.included = true;
    }
  }

  pub fn render<W: WriteJs>(&self, emitter: &mut Emitter<'_, W>) {
    let comment_node = ModuleItem::Stmt(Stmt::Empty(EmptyStmt {
      span: self.module_span,
    }));
    emitter.emit_module_item(&comment_node).unwrap();

    self.statements.iter().for_each(|stmt| {
      if stmt.included {
        emitter.emit_module_item(&stmt.node).unwrap();
      }
    });
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
      .field("statements", &self.statements)
      .field("definitions", &self.definitions)
      .finish()
  }
}

impl Hash for Module {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    state.write(self.id.as_bytes());
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
      .cloned()
      .unwrap_or_else(|| get_valid_name(nodejs_path::parse(&module.id).name).into());

    assert_ne!(&suggested_default_export_name, "default");

    assert!(!module
      .declared_symbols
      .contains_key(&suggested_default_export_name));
    module.declared_symbols.insert(
      suggested_default_export_name.clone(),
      *module.exports.get(&"default".into()).unwrap(),
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
