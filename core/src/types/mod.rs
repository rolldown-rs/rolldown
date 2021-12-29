use std::hash::Hash;

mod normalized_input_options;
pub use normalized_input_options::*;
mod normalized_output_options;
pub use normalized_output_options::*;

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct ResolvedId {
  pub id: String,
  pub external: bool,
}

impl ResolvedId {
  pub fn new(id: String, external: bool) -> Self {
    Self {
      id,
      external,
      // module_side_effects: false,
    }
  }
}

pub type ResolveIdResult = Option<ResolvedId>;

// --- UnresolvedModule

pub struct UnresolvedModule {
  pub file_name: Option<String>,
  pub id: String,
  pub importer: Option<String>,
  pub name: Option<String>,
}
