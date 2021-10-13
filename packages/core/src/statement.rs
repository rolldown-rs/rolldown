use std::{
  collections::HashSet,
  sync::{Arc, RwLock},
};

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
  pub node: ModuleItem,
  pub is_import_declaration: bool,
  pub is_export_declaration: bool,
  pub is_included: bool,
  pub defines: HashSet<String>,
  pub module_id: String,
  pub scope: Arc<Scope>,
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
    let scope = Arc::new(Scope::default());
    let s = Statement {
      defines,
      module_id,
      node,
      is_import_declaration,
      is_export_declaration,
      is_included: false,
      scope,
    };
    s.analyse();
    s
  }

  fn analyse(&self) {
    let mut statement_analyser = StatementAnalyser {
      scope: self.scope.clone(),
      new_scope: None,
      is_in_fn_context: false,
    };
    self.node.visit_children_with(&mut statement_analyser);
  }

  pub fn expand(this: &Arc<RwLock<Self>>) -> Vec<Arc<RwLock<Self>>> {
    let is_included_ref = &mut this.write().unwrap().is_included;
    if *is_included_ref {
      vec![]
    } else {
      *is_included_ref = true;
      vec![this.clone()]
    }
  }

  // pub fn get_node(&self) -> &ModuleItem {
  //   unsafe { Box::leak(Box::from_raw(self.node)) }
  // }

  // pub fn take_node(&self) -> ModuleItem {
  //   unsafe { *Box::from_raw(self.node) }
  // }

  fn replace_identifiers() {}
}

#[non_exhaustive]
pub struct StatementAnalyser {
  pub scope: Arc<Scope>,
  pub new_scope: Option<Arc<Scope>>,
  // we need is_in_fn_context to determined the block is belong to a function or just a independent block. 
  pub is_in_fn_context: bool,
}

impl StatementAnalyser {
  pub fn new(root_scope: Arc<Scope>) -> Self {
    StatementAnalyser {
      scope: root_scope,
      new_scope: None,
      is_in_fn_context: false,
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
      if let Some(parent) = new_scope.parent.as_ref() {
        self.scope = parent.clone()
      }
    }
  }
}

fn map_pat_to_string(pat: &Pat) -> Option<String> {
  match pat {
    Pat::Ident(ident) => Some(ident.id.sym.to_string()),
    _ => None,
  }
}

impl Visit for StatementAnalyser {
  fn visit_fn_expr(&mut self, node: &FnExpr, _parent: &dyn Node) {
    self.enter();
    let params = node
      .function
      .params
      .iter()
      .map(|p| map_pat_to_string(&p.pat))
      .flatten()
      .collect();
    self.new_scope = Some(Arc::new(Scope::new(
      Some(self.scope.clone()),
      params,
      false,
    )));
    if let Some(ident) = &node.ident {
      // named function expressions - the name is considered
      // part of the function's scope
      self
        .new_scope
        .as_ref()
        .unwrap()
        .add_declaration(&ident.sym.to_string(), false)
    }
    self.before_fold_children();
    self.is_in_fn_context = true;
    node.visit_children_with(self);
    self.leave();
  }

  fn visit_fn_decl(&mut self, node: &FnDecl, _parent: &dyn Node) {
    self.enter();
    self
      .scope
      .add_declaration(&node.ident.sym.to_string(), false);

    let params = node
      .function
      .params
      .iter()
      .map(|p| map_pat_to_string(&p.pat))
      .flatten()
      .collect();
    self.new_scope = Some(Arc::new(Scope::new(
      Some(self.scope.clone()),
      params,
      false,
    )));
    self.before_fold_children();
    self.is_in_fn_context = true;
    node.visit_children_with(self);
    self.leave();
  }

  fn visit_arrow_expr(&mut self, node: &ArrowExpr, _parent: &dyn Node) {
    self.enter();
    let params = node
      .params
      .iter()
      .map(|p| map_pat_to_string(p))
      .flatten()
      .collect();
    self.new_scope = Some(Arc::new(Scope::new(
      Some(self.scope.clone()),
      params,
      false,
    )));
    self.before_fold_children();
    self.is_in_fn_context = true;
    node.visit_children_with(self);
    self.leave();
  }
  fn visit_class_method(&mut self, node: &ClassMethod, _parent: &dyn Node) {
    self.enter();
    let params = node
      .function
      .params
      .iter()
      .map(|p| map_pat_to_string(&p.pat))
      .flatten()
      .collect();
    self.new_scope = Some(Arc::new(Scope::new(
      Some(self.scope.clone()),
      params,
      false,
    )));
    self.before_fold_children();
    self.is_in_fn_context = true;
    node.visit_children_with(self);
    self.leave();
  }

  fn visit_method_prop(
    &mut self,
    node: &MethodProp, 
    _parent: &dyn Node
  ) {
    self.enter();
    let params = node
      .function
      .params
      .iter()
      .map(|p| map_pat_to_string(&p.pat))
      .flatten()
      .collect();
    self.new_scope = Some(Arc::new(Scope::new(
      Some(self.scope.clone()),
      params,
      false,
    )));
    self.before_fold_children();
    self.is_in_fn_context = true;
    node.visit_children_with(self);
    self.leave();
  }

  fn visit_block_stmt(&mut self, node: &BlockStmt, _parent: &dyn Node) {
    self.enter();
    // check whether this block is belong to function
    // if yes. we don't need gennerate anothor scope for block stmt
    if self.is_in_fn_context {
      self.is_in_fn_context = false
    } else {
      self.new_scope = Some(Arc::new(Scope::new(
        Some(self.scope.clone()),
        vec![],
        true,
      )));
    }
    self.before_fold_children();
    node.visit_children_with(self);
    self.leave();
  }

  fn visit_catch_clause(&mut self, node: &CatchClause, _parent: &dyn Node) {
    // enter ---
    self.enter();
    // let params: Vec<String> = node.param.as_ref().map_or(vec![], |p| map_pat_to_string);
    let params: Vec<String> = vec![];
    self.new_scope = Some(Arc::new(Scope::new(
      Some(self.scope.clone()),
      params,
      true,
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
        let is_block_declaration = matches!(node.kind, VarDeclKind::Let | VarDeclKind::Const);
        self.scope.add_declaration(&name, is_block_declaration);
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
      .add_declaration(&node.ident.sym.to_string(), false);
    self.before_fold_children();
    node.visit_children_with(self);
    self.leave();
  }
}
