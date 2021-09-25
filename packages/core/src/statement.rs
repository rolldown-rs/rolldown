use std::{
  borrow::{Borrow, BorrowMut},
  collections::HashMap,
  panic::PanicInfo,
};

use swc_common::DUMMY_SP;
use swc_ecma_ast::*;
use swc_ecma_visit::{swc_ecma_ast::FnExpr, Fold, FoldWith, Node, Visit, VisitWith};

use crate::{ast::scope::Scope, module::Module, types::shared::Shared};

pub struct StatementOptions {}

#[derive(Debug, Clone, PartialEq)]
pub struct Statement {
  pub node: ModuleItem,
  // pub module: &'a Module,
  // pub index: i32,
  // pub id: String,
  // pub scope: Shared<Scope>,
  // pub defines: HashMap<String, bool>,
  // pub depends_on: HashMap<String, String>,
  // pub strongly_depends_on: HashMap<String, String>,
  pub is_included: bool,
  pub is_import_declaration: bool,
  pub is_export_declaration: bool,
  // pub modifies: HashMap<String, String>,
  // pub included: bool,
  // source: String,
  // margin:           { value: [0, 0] },
}

impl Statement {
  pub fn new(node: ModuleItem) -> Self {
    let is_import_declaration = if let ModuleItem::ModuleDecl(ModuleDecl::Import(_)) = &node {
      true
    } else {
      false
    };
    let is_export_declaration = if let ModuleItem::ModuleDecl(module_decl) = &node {
      match module_decl {
        ModuleDecl::ExportAll(_) => true,
        ModuleDecl::ExportDecl(_) => true,
        ModuleDecl::ExportDefaultDecl(_) => true,
        ModuleDecl::ExportDefaultExpr(_) => true,
        ModuleDecl::ExportNamed(_) => true,
        _ => false,
      }
    } else {
      false
    };
    // let id = module.id.clone() + "#" + &index.to_string();
    Statement {
      node,
      // module,
      // index,
      // id,
      // scope: Shared::new(Scope::new(None, None, false)),
      // defines: HashMap::new(),
      // depends_on: HashMap::new(),
      // strongly_depends_on: HashMap::new(),
      is_included: false,
      is_import_declaration,
      is_export_declaration,
    }
  }

  pub fn expand(this: &Shared<Self>) -> Shared<Self> {
    this.borrow_mut().is_included = true;
    this.clone()
  }

  // pub fn analyse(&mut self) {
  //   if self.is_import_declaration { return }

  //   let statement = self;
  //   let scope = statement.scope.clone();
  //   let mut statement_analyser = StatementAnalyser::new(scope.clone());
  //   statement.node.visit_children_with(&mut statement_analyser);

  //   scope.declarations.keys().for_each(|name| {
  //     statement.defines.insert(name.to_owned(), true);
  //   })
  // }

  // pub fn expand(&mut self) {
  //   self.is_included = true;
  //   let reuslt = vec![];
  //   let dependencies = self.depends_on.keys();
  //   dependencies.for_each(|name| {
  //     if self.defines.contains_key(name) { return }
  //     self.module
  //   })

  // }
}

struct StatementAnalyser {
  pub scope: Shared<Scope>,
  pub new_scope: Option<Shared<Scope>>,
}

impl StatementAnalyser {
  pub fn new(root_scope: Shared<Scope>) -> Self {
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
      self.scope = new_scope.borrow().parent.as_ref().unwrap().clone()
    }
  }
}

impl Visit for StatementAnalyser {
  fn visit_fn_expr(&mut self, node: &FnExpr, _parent: &dyn Node) {
    self.enter();
    let params = node.function.params.iter().map(|p| p.pat.clone()).collect();
    self.new_scope = Some(Shared::new(Scope::new(
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
      .borrow_mut()
      .add_declaration(&node.ident.sym.to_string(), Decl::Fn(node.clone()));
    let params = node.function.params.iter().map(|p| p.pat.clone()).collect();
    self.new_scope = Some(Shared::new(Scope::new(
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
    self.new_scope = Some(Shared::new(Scope::new(
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
    self.new_scope = Some(Shared::new(Scope::new(
      Some(self.scope.clone()),
      None,
      true,
    )));

    self.before_fold_children();
    node.visit_children_with(self);
    self.leave();
  }

  fn visit_catch_clause(&mut self, node: &CatchClause, _parent: &dyn Node) {
    // enter ---
    self.enter();
    let params: Vec<Pat> = node.param.as_ref().map_or(vec![], |p| vec![p.clone()]);
    self.new_scope = Some(Shared::new(Scope::new(
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
        self
          .scope
          .borrow_mut()
          .add_declaration(&name, Decl::Var(node.clone()));
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
      .borrow_mut()
      .add_declaration(&node.ident.sym.to_string(), Decl::Class(node.clone()));
    self.before_fold_children();
    node.visit_children_with(self);
    self.leave();
  }
}
