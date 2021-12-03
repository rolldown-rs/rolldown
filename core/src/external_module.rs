use std::{collections::BTreeSet, hash::Hash};

use crate::types::{shared, Shared};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ExternalModule {
  pub id: String,
  pub importers: BTreeSet<String>,
  pub dynamic_importers: BTreeSet<String>,
}
impl ExternalModule {
  pub fn new(id: String) -> Shared<Self> {
    shared(ExternalModule {
      id,
      importers: BTreeSet::default(),
      dynamic_importers: BTreeSet::default(),
    })
  }
}

impl Hash for ExternalModule {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    state.write(&self.id.as_bytes());
  }
}
