use std::collections::HashMap;

use swc_atoms::JsWord;
use swc_common::Mark;
use swc_ecma_ast::VarDeclKind;

use super::Scanner;

impl Scanner {
  #[inline]
  pub fn push_scope(&mut self, kind: ScopeKind) {
    // let scope = Scope::new(kind, );
    let scope = Scope::new(kind);
    self.stacks.push(scope);
  }

  #[inline]
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
  pub declared_symbols: HashMap<JsWord, Mark>,
  pub declared_symbols_kind: HashMap<JsWord, BindType>,
}

impl Scope {
  pub fn new(kind: ScopeKind) -> Self {
    Self {
      // depth,
      kind,
      declared_symbols: Default::default(),
      declared_symbols_kind: Default::default(),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindType {
  Var,
  Let,
  Const,
  Import,
}

impl From<VarDeclKind> for BindType {
  fn from(var_decl_kind: VarDeclKind) -> Self {
    match var_decl_kind {
      VarDeclKind::Const => Self::Const,
      VarDeclKind::Let => Self::Let,
      VarDeclKind::Var => Self::Let,
    }
  }
}
