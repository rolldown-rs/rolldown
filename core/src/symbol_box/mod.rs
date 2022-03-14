use ena::unify::{InPlaceUnificationTable, UnifyKey};
use once_cell::sync::Lazy;
use swc_common::{Globals, Mark, GLOBALS};

#[derive(Debug)]
pub struct SymbolBox {
  pub mark_uf: InPlaceUnificationTable<MarkIndex>,
  // globals: Globals,
}

pub(crate) static SYMBOL_GLOBALS: Lazy<Globals> = Lazy::new(Globals::new);

impl Default for SymbolBox {
  fn default() -> Self {
    Self::new()
  }
}

impl SymbolBox {
  pub fn new() -> Self {
    let mut mark_uf: InPlaceUnificationTable<MarkIndex> = Default::default();
    GLOBALS.set(&SYMBOL_GLOBALS, || {
      // Mark(0) is a special mark in SWC. we need to drop it.
      mark_uf.new_key(());
    });
    Self {
      // globals,
      mark_uf,
    }
  }

  pub fn new_mark(&mut self) -> Mark {
    GLOBALS.set(&SYMBOL_GLOBALS, || {
      // Mark(0) is a special mark in SWC. we need to drop it.
      self.mark_uf.new_key(()).as_mark()
    })
  }

  pub fn union(&mut self, a: Mark, b: Mark) {
    self.mark_uf.union(a, b)
  }

  pub fn unioned(&mut self, a: Mark, b: Mark) -> bool {
    self.mark_uf.unioned(a, b)
  }

  pub fn find_root(&mut self, a: Mark) -> Mark {
    self.mark_uf.find(a).into()
  }
}

#[derive(Debug, PartialEq, Eq, Hash, Default, Clone, Copy)]
pub struct MarkIndex(Mark);

impl MarkIndex {
  #[inline]
  pub fn as_mark(self) -> Mark {
    self.0
  }
}

impl UnifyKey for MarkIndex {
  type Value = ();

  fn index(&self) -> u32 {
    self.0.as_u32()
  }

  fn from_index(u: u32) -> Self {
    Self(Mark::from_u32(u))
  }

  fn tag() -> &'static str {
    "Mark"
  }
}

impl From<Mark> for MarkIndex {
  fn from(m: Mark) -> Self {
    Self(m)
  }
}

impl From<MarkIndex> for Mark {
  fn from(m: MarkIndex) -> Self {
    m.0
  }
}
