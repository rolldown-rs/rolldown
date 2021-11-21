use std::{cell::RefCell, rc::Rc};

pub mod module;

pub type Shared<T> = Rc<RefCell<T>>;
pub fn shared<T>(item: T) -> Shared<T> {
  Rc::new(RefCell::new(item))
}
