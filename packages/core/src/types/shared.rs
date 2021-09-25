use std::cell::{Ref, RefCell, RefMut};
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Clone, PartialEq)]
pub struct Shared<T> {
  v: Rc<RefCell<T>>,
}

impl<T> Shared<T> {
  pub fn new(t: T) -> Shared<T> {
    Shared {
      v: Rc::new(RefCell::new(t)),
    }
  }
}
impl<T> Shared<T> {
  pub fn borrow(&self) -> Ref<T> {
    self.v.borrow()
  }

  pub fn borrow_mut(&self) -> RefMut<T> {
    self.v.borrow_mut()
  }

  pub fn as_ptr(&self) -> *mut T {
    self.v.as_ptr()
  }
}

impl<T: fmt::Display> fmt::Display for Shared<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.deref())
  }
}

impl<T: fmt::Debug> fmt::Debug for Shared<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{:?}", self.deref())
  }
}

impl<'a, T> Deref for Shared<T> {
  type Target = T;

  #[inline]
  fn deref(&self) -> &T {
    unsafe { self.as_ptr().as_ref().unwrap() }
  }
}

/*
// Cute, but useless, since it needs to be mutable
// and so can't be shared anyway
impl <'a, T> DerefMut for Shared<T>
{   #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe {self.as_ptr().as_mut().unwrap()}
    }
}
*/
