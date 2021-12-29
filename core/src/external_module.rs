use std::hash::Hash;

use crate::graph::DepNode;

#[derive(PartialEq, Eq, Clone)]
pub struct ExternalModule {
  pub id: String,
  pub module_side_effects: bool,
}
impl ExternalModule {
  pub fn new(id: String) -> Self {
    ExternalModule {
      id,
      module_side_effects: true,
    }
  }
}

impl std::fmt::Debug for ExternalModule {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("ExternalModule").field(&self.id).finish()
  }
}

impl Hash for ExternalModule {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    state.write(&self.id.as_bytes());
  }
}

impl Into<DepNode> for ExternalModule {
  fn into(self) -> DepNode {
    DepNode::Ext(self)
  }
}
