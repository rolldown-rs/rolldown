use std::sync::{Arc, Mutex};

use crate::types::ResolveIdResult;

pub struct PluginDriver {
  pub plugins: Arc<Mutex<Vec<Box<dyn Plugin + Send>>>>,
}

impl PluginDriver {
  pub fn new() -> Self {
    Self {
      plugins: Default::default(),
    }
  }

  pub fn from_plugins(plugins: Arc<Mutex<Vec<Box<dyn Plugin + Send>>>>) -> Self {
    Self { plugins }
  }
}

// Align to https://rollupjs.org/guide/en/#build-hooks

impl PluginDriver {
  #[inline]
  pub fn resolve_id(&self, source: &str, importer: Option<&str>) -> ResolveIdResult {
    let result = self
      .plugins
      .lock()
      .unwrap()
      .iter_mut()
      .map(|plugin| plugin.resolve_id(source, importer))
      .take_while(|result| result.is_some())
      .map(|r| r.unwrap())
      .next();

    result
  }

  #[inline]
  pub fn load(&self, id: &str) -> Option<String> {
    let result = self
      .plugins
      .lock()
      .unwrap()
      .iter_mut()
      .map(|plugin| plugin.load(id))
      .take_while(|result| result.is_some())
      .map(|r| r.unwrap())
      .next();

    result
  }

  pub fn transform(&self, _code: String, _id: &str) -> Option<String> {
    None
  }
}

pub trait Plugin {
  // Align to https://rollupjs.org/guide/en/#build-hooks

  fn get_name(&self) -> &'static str;

  #[inline]
  fn resolve_id(&mut self, _source: &str, _importer: Option<&str>) -> ResolveIdResult {
    //  async, first
    None
  }

  #[inline]
  fn load(&mut self, _id: &str) -> Option<String> {
    // async, first
    None
  }
}
