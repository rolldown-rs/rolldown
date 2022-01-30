use std::hash::Hash;

mod normalized_input_options;
pub use normalized_input_options::*;
mod normalized_output_options;
pub use normalized_output_options::*;
use smol_str::SmolStr;

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct ResolvedId {
  pub id: SmolStr,
  pub external: bool,
}

impl ResolvedId {
  pub fn new(id: SmolStr, external: bool) -> Self {
    Self {
      id,
      external,
      // module_side_effects: false,
    }
  }
}

pub type ResolveIdResult = Option<ResolvedId>;