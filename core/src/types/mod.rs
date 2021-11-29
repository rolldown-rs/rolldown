use std::{cell::RefCell, rc::Rc};

mod mod_or_ext;
pub use mod_or_ext::*;

mod normalized_input_options;
pub use normalized_input_options::*;
mod normalized_output_options;
pub use normalized_output_options::*;

// --- shared

pub type Shared<T> = Rc<RefCell<T>>;
#[inline]
pub fn shared<T>(item: T) -> Shared<T> {
  Rc::new(RefCell::new(item))
}

// --- ResolvedId

#[derive(Clone, Debug)]
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
