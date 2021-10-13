use std::{collections::HashSet, sync::Arc};

use swc_common::sync::RwLock;
use swc_ecma_ast::*;
use swc_ecma_visit::{swc_ecma_ast::FnExpr, Node, Visit, VisitWith};

use crate::ast::scope::Scope;

pub struct StatementOptions {}

fn collect_defines(node: &ModuleItem) -> HashSet<String> {
  let mut defines = HashSet::new();
  if let ModuleItem::Stmt(Stmt::Decl(decl)) = node {
    match decl {
      Decl::Class(node) => {
        defines.insert(node.ident.sym.to_string());
      }
      Decl::Fn(node) => {
        defines.insert(node.ident.sym.to_string());
      }
      Decl::Var(node) => {
        node.decls.iter().for_each(|decl| {
          match &decl.name {
            Pat::Ident(ident) => {
              defines.insert(ident.id.sym.to_string());
            }
            _ => {}
          };
        });
      }
      _ => {}
    }
  };
  defines
}

#[derive(Debug)]
#[non_exhaustive]
pub struct Statement {
  node: *mut ModuleItem,
  pub is_import_declaration: bool,
  pub is_export_declaration: bool,
  pub is_included: RwLock<bool>,
  pub defines: HashSet<String>,
  pub module_id: String,
}

unsafe impl Send for Statement {}
unsafe impl Sync for Statement {}

impl Statement {
  pub fn new(node: ModuleItem, module_id: String) -> Self {
    let is_import_declaration = matches!(&node, ModuleItem::ModuleDecl(ModuleDecl::Import(_)));
    let is_export_declaration = if let ModuleItem::ModuleDecl(module_decl) = &node {
      matches!(
        module_decl,
        ModuleDecl::ExportAll(_)
          | ModuleDecl::ExportDecl(_)
          | ModuleDecl::ExportDefaultDecl(_)
          | ModuleDecl::ExportDefaultExpr(_)
          | ModuleDecl::ExportNamed(_)
      )
    } else {
      false
    };
    let defines = collect_defines(&node);
    Statement {
      defines,
      module_id,
      node: Box::into_raw(Box::new(node)),
      is_import_declaration,
      is_export_declaration,
      is_included: RwLock::new(false),
    }
  }

  pub fn get_node(&self) -> &ModuleItem {
    unsafe { Box::leak(Box::from_raw(self.node)) }
  }

  pub fn take_node(&self) -> ModuleItem {
    unsafe { *Box::from_raw(self.node) }
  }
}

#[non_exhaustive]
pub struct StatementAnalyser {
  pub scope: Arc<Scope>,
  pub new_scope: Option<Arc<Scope>>,
}

impl StatementAnalyser {
  pub fn new(root_scope: Arc<Scope>) -> Self {
    StatementAnalyser {
      scope: root_scope,
      new_scope: None,
    }
  }

  pub fn enter(&mut self) {
    self.new_scope.take();
  }

  pub fn before_fold_children(&mut self) {
    if let Some(ref new_scope) = self.new_scope {
      self.scope = new_scope.clone()
    }
  }

  pub fn leave(&mut self) {
    if let Some(new_scope) = &self.new_scope {
      self.scope = new_scope.parent.as_ref().unwrap().clone()
    }
  }
}

impl Visit for StatementAnalyser {
  fn visit_fn_expr(&mut self, node: &FnExpr, _parent: &dyn Node) {
    self.enter();
    let params = node.function.params.iter().map(|p| p.pat.clone()).collect();
    self.new_scope = Some(Arc::new(Scope::new(
      Some(self.scope.clone()),
      Some(params),
      false,
    )));
    // if let Some(ident) = node.ident {
    //   // TODO:
    //   // named function expressions - the name is considered
    //   // part of the function's scope
    //   new_scope.add_declaration(ident.sym.to_string(), declaration)
    // }
    self.before_fold_children();
    node.visit_children_with(self);
    self.leave();
  }

  fn visit_fn_decl(&mut self, node: &FnDecl, _parent: &dyn Node) {
    self.enter();
    self
      .scope
      .add_declaration(&node.ident.sym.to_string(), Decl::Fn(node.clone()));
    let params = node.function.params.iter().map(|p| p.pat.clone()).collect();
    self.new_scope = Some(Arc::new(Scope::new(
      Some(self.scope.clone()),
      Some(params),
      false,
    )));
    self.before_fold_children();
    node.visit_children_with(self);
    self.leave();
  }

  fn visit_arrow_expr(&mut self, node: &ArrowExpr, _parent: &dyn Node) {
    self.enter();
    self.new_scope = Some(Arc::new(Scope::new(
      Some(self.scope.clone()),
      Some(node.params.clone()),
      false,
    )));
    self.before_fold_children();
    node.visit_children_with(self);
    self.leave();
  }

  fn visit_block_stmt(&mut self, node: &BlockStmt, _parent: &dyn Node) {
    // enter ---
    self.enter();

    // TODO: should check whether this block is belong to function
    self.new_scope = Some(Arc::new(Scope::new(Some(self.scope.clone()), None, true)));

    self.before_fold_children();
    node.visit_children_with(self);
    self.leave();
  }

  fn visit_catch_clause(&mut self, node: &CatchClause, _parent: &dyn Node) {
    // enter ---
    self.enter();
    let params: Vec<Pat> = node.param.as_ref().map_or(vec![], |p| vec![p.clone()]);
    self.new_scope = Some(Arc::new(Scope::new(
      Some(self.scope.clone()),
      Some(params),
      false,
    )));
    self.before_fold_children();
    node.visit_children_with(self);
    self.leave();
  }

  fn visit_var_decl(&mut self, node: &VarDecl, _parent: &dyn Node) {
    self.enter();
    node.decls.iter().for_each(|declarator| {
      if let Pat::Ident(binding_ident) = &declarator.name {
        let name = binding_ident.id.sym.to_string();
        self.scope.add_declaration(&name, Decl::Var(node.clone()));
      };
    });
    self.before_fold_children();
    node.visit_children_with(self);
    self.leave();
  }

  fn visit_class_decl(&mut self, node: &ClassDecl, _parent: &dyn Node) {
    // enter ---
    self.enter();
    self
      .scope
      .add_declaration(&node.ident.sym.to_string(), Decl::Class(node.clone()));
    self.before_fold_children();
    node.visit_children_with(self);
    self.leave();
  }
}
