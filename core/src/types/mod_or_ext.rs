use std::hash::Hash;

use crate::{external_module::ExternalModule, Module};

use super::Shared;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModOrExt {
  Mod(Shared<Module>),
  Ext(Shared<ExternalModule>),
}

impl std::convert::From<Shared<ExternalModule>> for ModOrExt {
  fn from(ext: Shared<ExternalModule>) -> Self {
    ModOrExt::Ext(ext)
  }
}

impl std::convert::From<Shared<Module>> for ModOrExt {
  fn from(m: Shared<Module>) -> Self {
    ModOrExt::Mod(m)
  }
}

impl Hash for ModOrExt {
  // TODO: Is the implamentation safe?
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    match self {
      ModOrExt::Mod(m) => {
        state.write(&m.borrow().id.as_bytes());
      }
      ModOrExt::Ext(m) => {
        state.write(&m.borrow().id.as_bytes());
      }
    }
  }
}

impl ModOrExt {
  #[inline]
  pub fn is_mod(&self) -> bool {
    matches!(self, ModOrExt::Mod(_))
  }

  #[inline]
  pub fn is_ext(&self) -> bool {
    !self.is_mod()
  }

  pub fn into_mod(self) -> Option<Shared<Module>> {
    if let ModOrExt::Mod(m) = self {
      Some(m)
    } else {
      None
    }
  }

  pub fn into_ext(self) -> Option<Shared<ExternalModule>> {
    if let ModOrExt::Ext(m) = self {
      Some(m)
    } else {
      None
    }
  }

  pub fn add_importers(&self, id: String) {
    match self {
      ModOrExt::Mod(m) => {
        m.borrow_mut().importers.insert(id);
      }
      ModOrExt::Ext(m) => {
        m.borrow_mut().importers.insert(id);
      }
    }
  }

  pub fn add_dynamic_importers(&self, id: String) {
    match self {
      ModOrExt::Mod(m) => {
        m.borrow_mut().dynamic_importers.insert(id);
      }
      ModOrExt::Ext(m) => {
        m.borrow_mut().dynamic_importers.insert(id);
      }
    }
  }
}
