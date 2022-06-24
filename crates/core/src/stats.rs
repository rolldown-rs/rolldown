use crate::Asset;

#[derive(Debug)]
pub struct Stats {
  assets: Vec<Asset>,
}

impl Stats {
  pub fn new(assets: Vec<Asset>) -> Self {
    Self { assets }
  }
}

impl Stats {
  pub fn assets(&self) -> &[Asset] {
    &self.assets
  }
}
