use crate::types::{shared, Shared};

#[derive(Debug, Clone, PartialEq)]
pub struct ExternalModule {
  pub name: String,
}
impl ExternalModule {
  pub fn new(name: String) -> Shared<Self> {
    shared(ExternalModule { name })
  }
}
