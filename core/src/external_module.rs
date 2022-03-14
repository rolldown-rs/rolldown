use std::hash::Hash;

// use crate::graph::DepNode;

#[derive(Clone, Debug)]
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

impl Hash for ExternalModule {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    state.write(self.id.as_bytes());
  }
}
