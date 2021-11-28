use std::{
  collections::HashSet,
  sync::{
    atomic::{AtomicBool},
    Arc,
  },
};

use ahash::RandomState;
use swc_common::sync::RwLock;
use swc_ecma_ast::*;
use swc_ecma_visit::VisitWith;

use crate::{ast::scope::Scope};

pub mod analyser;
#[cfg(test)]
pub mod tests;

pub struct StatementOptions {}

#[non_exhaustive]
pub struct Statement {
  pub module_id: String,
  pub node: RwLock<ModuleItem>,
  pub is_import_declaration: bool,
  pub is_export_declaration: bool,
  pub is_included: Arc<AtomicBool>,
  pub defines: HashSet<String, RandomState>,
  pub modifies: HashSet<String, RandomState>,
  pub depends_on: HashSet<String, RandomState>,
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
    // let defines = collect_defines(&node);
    // println!("defines: {:?}", defines);
    let scope = Arc::new(Scope::default());
    let mut s = Statement {
      module_id,
      defines: HashSet::default(),
      node: RwLock::new(node),
      is_import_declaration,
      is_export_declaration,
      is_included: Arc::new(AtomicBool::new(false)),
      depends_on: HashSet::default(),
      modifies: HashSet::default(),
      scope,
    };
    s.analyse();
    s
  }

  fn analyse(&mut self) {
    let mut statement_analyser = analyser::StatementAnalyser::new(self.scope.clone());
    self
      .node
      .read()
      .visit_children_with(&mut statement_analyser);
    self.defines = statement_analyser.scope.defines.read().clone();
    self.depends_on = statement_analyser.depends_on.clone();
    // consider all depends as modifies for now, even they are only read-only.
    self.modifies = statement_analyser.depends_on;
    // debug!("defines: {:?}, scope defines: {:?}", self.defines, self.scope.defines);
  }

  // pub fn expand(self: &Arc<Self>, module: &Module, graph: &graph::Graph) -> Vec<Arc<Self>> {
  //   if self.is_included.swap(true, Ordering::SeqCst) {
  //     return vec![];
  //   }

  //   let mut result = vec![];

  //   log::debug!(
  //     "expand statement depends on {:?} in module {}",
  //     self.depends_on,
  //     module.id
  //   );

  //   // We have a statement, and it hasn't been included yet. First, include
  //   // the statements it depends on
  //   self.depends_on.iter().for_each(|name| {
  //     if !self.defines.contains(name) {
  //       // The name doesn't belong to this statement, we need to search it in module.
  //       result.append(&mut module.define(name, graph));
  //     }
  //   });

  //   // include the statement itself
  //   result.push(self.clone());

  //   // then include any statements that could modify the
  //   self.defines.iter().for_each(|name| {
  //     if let Some(modifications) = module.modifications.get(name) {
  //       modifications.iter().for_each(|statement| {
  //         result.append(&mut statement.expand(module, graph));
  //       });
  //     }
  //   });

  //   result
  // }
}
