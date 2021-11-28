use std::collections::HashSet;

use crate::types::{shared, Shared};

#[derive(Clone, Debug)]
pub struct ExternalModule {
  pub id: String,
  pub importers: HashSet<String>,
  pub dynamic_importers: HashSet<String>,
}
impl ExternalModule {
  pub fn new(id: String) -> Shared<Self> {
    shared(ExternalModule {
      id,
      importers: HashSet::default(),
      dynamic_importers: HashSet::default(),
    })
  }
}
