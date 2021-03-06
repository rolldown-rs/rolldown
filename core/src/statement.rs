use std::collections::{HashMap, HashSet};

use swc_atoms::JsWord;
use swc_common::Mark;
use swc_ecma_ast::ModuleItem;

use crate::utils::side_effect::SideEffect;

#[derive(Clone, PartialEq, Eq)]
pub struct Statement {
  pub node: ModuleItem,
  pub included: bool,
  pub declared: HashMap<JsWord, Mark>,
  pub reads: HashSet<Mark>,
  pub writes: HashSet<Mark>,
  pub side_effect: Option<SideEffect>,
}

impl Statement {
  pub fn new(node: ModuleItem) -> Self {
    Self {
      node,
      included: false,
      declared: Default::default(),
      reads: Default::default(),
      writes: Default::default(),
      side_effect: Default::default(),
    }
  }

  #[inline]
  pub fn include(&mut self) {
    self.included = true;
  }
}

impl std::fmt::Debug for Statement {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Statement")
      .field("included", &self.included)
      .field("declared", &self.declared)
      .field("reads", &self.reads)
      .field("writes", &self.writes)
      .field("side_effect", &self.side_effect)
      .finish()
  }
}
