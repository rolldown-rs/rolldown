

use crate::{
  module_loader::ResolvedId,
  types::{shared, ResolveIdResult, Shared},
};



pub struct PluginDriver {}

impl PluginDriver {
  pub fn new() -> Shared<Self> {
    shared(Self {})
  }
}

impl PluginDriver {
  // build hooks
  pub fn options() {}

  pub fn build_start(&self) {}

  #[inline]
  pub fn resolve_id(&self, _source: &str, _importer: Option<&str>) -> Option<ResolvedId> {
    None
  }

  #[inline]
  pub fn load(&self, _id: &str) -> Option<String> {
    // TODO: call hook load of plugins
    None
  }

  pub fn transform(&self, code: String, _id: &str) -> String {
    code
  }

  pub fn module_parsed(&self) {}

  pub fn resolve_dynamic_import(&self, _specifier: &str, _importer: &str) -> ResolveIdResult {
    // TODO:
    None
  }

  pub fn build_end(&self) {}
}
