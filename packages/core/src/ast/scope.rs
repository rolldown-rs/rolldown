use std::collections::HashMap;
use std::sync::Arc;

use swc_common::sync::RwLock;
use swc_ecma_ast::{Decl, Pat, VarDeclKind};

#[derive(Debug)]
pub struct Scope {
  pub parent: Option<Arc<Scope>>,
  pub depth: u32,
  pub declarations: RwLock<HashMap<String, Decl>>,
  pub is_block_scope: bool,
}

impl Scope {
  pub fn new(parent: Option<Arc<Scope>>, params: Option<Vec<Pat>>, block: bool) -> Scope {
    let _declarations = params.as_ref().map_or(HashMap::new(), |params| {
      let mut declarations = HashMap::new();
      params.iter().for_each(|p| {
        if let Pat::Ident(binding_ident) = &p {
          declarations.insert(binding_ident.id.sym.to_string(), params);
        }
      });
      declarations
    });
    Scope {
      depth: parent.as_ref().map_or(0, |p| p.depth + 1),
      parent,
      declarations: RwLock::new(HashMap::new()),
      is_block_scope: block,
    }
  }

  pub fn add_declaration(&self, name: &str, declaration: Decl) {
    let is_block_declaration = if let Decl::Var(var_decl) = &declaration {
      matches!(var_decl.kind, VarDeclKind::Let | VarDeclKind::Const)
    } else {
      false
    };

    if !is_block_declaration && self.is_block_scope {
      self
        .parent
        .as_ref()
        .unwrap()
        .add_declaration(name, declaration)
    } else {
      self
        .declarations
        .write()
        .insert(name.to_owned(), declaration);
    }
  }

  pub fn get_declaration(&self, name: &str) -> Option<Decl> {
    let read_lock = self.declarations.read();
    if read_lock.contains_key(name) {
      return read_lock.get(name).map(|d| d.clone());
    }
    if let Some(parent) = &self.parent {
      parent.get_declaration(name)
    } else {
      None
    }
  }

  pub fn contains(&self, name: &str) -> bool {
    self.get_declaration(name).is_some()
  }

  pub fn find_defining_scope(&self, name: &str) -> Option<&Self> {
    if self.declarations.read().contains_key(name) {
      return Some(self);
    }

    if let Some(parent) = &self.parent {
      parent.find_defining_scope(name)
    } else {
      None
    }
  }
}
