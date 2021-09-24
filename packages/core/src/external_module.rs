use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct ExternalModule {
  pub name: String,
}
impl ExternalModule {
  pub fn new(name: String) -> Self {
    ExternalModule { name }
  }
}
