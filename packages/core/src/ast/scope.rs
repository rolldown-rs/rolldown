use std::collections::HashSet;
use std::sync::Arc;

use ahash::RandomState;
use swc_common::sync::RwLock;

#[derive(Debug)]
pub struct Scope {
  pub parent: Option<Arc<Scope>>,
  pub depth: u32,
  // FIXME: collected defines has empty string ""
  pub defines: RwLock<HashSet<String, RandomState>>,
  pub is_block_scope: bool,
}

impl Default for Scope {
  fn default() -> Self {
    Scope {
      parent: None,
      depth: 0,
      defines: RwLock::new(HashSet::default()),
      is_block_scope: false,
    }
  }
}

impl Scope {
  pub fn new(parent: Option<Arc<Scope>>, params: Vec<String>, block: bool) -> Scope {
    let mut defines = HashSet::default();
    params.into_iter().for_each(|p| {
      defines.insert(p);
    });
    let depth = parent.as_ref().map_or(0, |p| p.depth + 1);
    Scope {
      depth,
      parent,
      defines: RwLock::new(defines),
      is_block_scope: block,
    }
  }

  pub fn add_declaration(&self, name: &str, is_block_declaration: bool) {
    if !is_block_declaration && self.is_block_scope {
      self
        .parent
        .as_ref()
        .unwrap_or_else(|| panic!("parent not found for name {:?}", name))
        .add_declaration(name, is_block_declaration)
    } else {
      self.defines.write().insert(name.to_owned());
    }
  }

  pub fn contains(&self, name: &str) -> bool {
    if self.defines.read().contains(name) {
      true
    } else if let Some(parent) = self.parent.as_ref() {
      parent.contains(name)
    } else {
      false
    }
  }

  pub fn find_defining_scope(self: &Arc<Self>, name: &str) -> Option<Arc<Self>> {
    if self.defines.read().contains(name) {
      Some(self.clone())
    } else if let Some(parent) = self.parent.as_ref() {
      parent.find_defining_scope(name)
    } else {
      None
    }
  }
}
