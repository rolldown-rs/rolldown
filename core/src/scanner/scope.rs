use std::collections::HashMap;

use swc_atoms::JsWord;
use swc_common::{Mark, SyntaxContext};

use super::Scanner;

impl Scanner {
  pub fn get_cur_scope(&self) -> &Scope {
    self.stacks.last().unwrap()
  }

  pub fn into_cur_scope(self) -> Scope {
    self.stacks.into_iter().next().unwrap()
  }

  pub fn get_cur_scope_mut(&mut self) -> &mut Scope {
    self.stacks.last_mut().unwrap()
  }

  pub fn push_scope(&mut self, kind: ScopeKind) {
    // let scope = Scope::new(kind, );
    let scope = Scope::new(kind, Mark::fresh(Mark::root()));
    self.stacks.push(scope);
  }

  pub fn pop_scope(&mut self) {
    self.stacks.pop();
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScopeKind {
  Block,
  Fn,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
  // pub depth: usize,
  pub kind: ScopeKind,
  pub mark: Mark,
  pub declared_symbols: HashMap<JsWord, SyntaxContext>,
}

impl Scope {
  pub fn new(kind: ScopeKind, mark: Mark) -> Self {
    Self {
      // depth,
      kind,
      mark,
      declared_symbols: Default::default(),
    }
  }
}
