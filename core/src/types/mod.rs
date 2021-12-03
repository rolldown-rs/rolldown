use std::{
  cell::{Ref, RefCell, RefMut},
  hash::Hash,
  rc::Rc,
};

mod mod_or_ext;
pub use mod_or_ext::*;

mod normalized_input_options;
pub use normalized_input_options::*;
mod normalized_output_options;
pub use normalized_output_options::*;

use crate::Module;

// --- shared

// pub type Shared<T> = Rc<RefCell<T>>;
#[derive(Debug, PartialEq, Eq)]
pub struct Shared<T>(Rc<RefCell<T>>);
#[inline]
pub fn shared<T>(item: T) -> Shared<T> {
  Shared(Rc::new(RefCell::new(item)))
}

impl<T> Shared<T> {
  pub fn new(t: T) -> Shared<T> {
    Shared(Rc::new(RefCell::new(t)))
  }
}

impl<T> Shared<T> {
  pub fn borrow(&self) -> Ref<T> {
    self.0.borrow()
  }

  pub fn borrow_mut(&self) -> RefMut<T> {
    self.0.borrow_mut()
  }

  pub fn as_ptr(&self) -> *mut T {
    self.0.as_ptr()
  }
}

impl<T> Clone for Shared<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl Hash for Shared<Module> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    state.write(self.0.borrow().id.as_bytes());
  }
}

// impl <T: std::fmt::Display> std::fmt::Display for Shared<T> {
//   fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//       write!(f, "{}", self.deref())
//   }
// }

// impl <T: std::fmt::Debug> std::fmt::Debug for Shared<T> {
//   fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//       write!(f, "{:?}", self.deref())
//   }
// }

// --- ResolvedId

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
