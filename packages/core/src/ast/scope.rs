use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Weak};

use swc_common::sync::RwLock;
use swc_ecma_ast::{Decl, Pat, VarDeclKind};

#[derive(Debug)]
pub struct Scope {
  pub parent: Weak<Scope>,
  pub depth: u32,
  pub defines: RwLock<HashSet<String>>,
  pub is_block_scope: bool,
}

impl Default for Scope {
  fn default() -> Self {
    Scope {
      parent: Weak::default(),
      depth: 0,
      defines: RwLock::new(HashSet::default()),
      is_block_scope: false,
    }
  }
}

impl Scope {
  pub fn new(parent: Weak<Scope>, params: Vec<String>, block: bool) -> Scope {
    let mut defines = HashSet::new();
    params.into_iter().for_each(|p| {
      defines.insert(p);
    });
    let depth = parent.upgrade().map_or(0, |p| p.depth + 1);
    Scope {
      depth,
      parent,
      defines: RwLock::new(defines),
      is_block_scope: block,
    }
  }

  pub fn add_declaration(&self, name: &str, is_block_declaration: bool) {
    // let is_block_declaration = if let Decl::Var(var_decl) = &is_block_declaration {
    //   matches!(var_decl.kind, VarDeclKind::Let | VarDeclKind::Const)
    // } else {
    //   false
    // };

    if !is_block_declaration && self.is_block_scope {
      self
        .parent
        .upgrade()
        .unwrap()
        .add_declaration(name, is_block_declaration)
    } else {
      self.defines.write().insert(name.to_owned());
    }
  }

  // pub fn get_declaration(&self, name: &str) -> Option<Decl> {
  //   let read_lock = self.declarations.read();
  //   if read_lock.contains_key(name) {
  //     return read_lock.get(name).cloned();
  //   }
  //   if let Some(parent) = &self.parent {
  //     parent.get_declaration(name)
  //   } else {
  //     None
  //   }
  // }

  pub fn contains(&self, name: &str) -> bool {
    if self.defines.read().contains(name) {
      true
    } else if let Some(parent) = self.parent.upgrade() {
      parent.contains(name)
    } else {
      false
    }
  }

  pub fn find_defining_scope(self: &Arc<Self>, name: &str) -> Option<Arc<Self>> {
    if self.defines.read().contains(name) {
      Some(self.clone())
    } else if let Some(parent) = self.parent.upgrade() {
      parent.find_defining_scope(name)
    } else {
      None
    }
  }
}
