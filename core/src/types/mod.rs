use std::{cell::RefCell, rc::Rc};

use crate::module_loader::ResolvedId;

pub mod module;

pub type Shared<T> = Rc<RefCell<T>>;
pub fn shared<T>(item: T) -> Shared<T> {
  Rc::new(RefCell::new(item))
}

pub type ResolveIdResult = Option<ResolvedId>;
