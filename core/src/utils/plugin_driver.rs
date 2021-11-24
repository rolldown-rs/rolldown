use std::path::Path;

use crate::types::{shared, Shared};

use super::nodejs;

pub struct PluginDriver {}

impl PluginDriver {
  pub fn new() -> Shared<Self> {
    shared(Self {})
  }
}

enum ResolvedId {}

impl PluginDriver {
  // build hooks
  pub fn options() {}

  pub fn build_start(&self) {}

  #[inline]
  pub fn resolve_id(&self, source: &str, importer: Option<&str>) -> Option<String> {
    let source = Path::new(source).to_path_buf();
    let mut id = if source.is_absolute() {
      source
    } else if importer.is_none() {
      nodejs::resolve(&source)
    } else {
      let is_normal_import = source.starts_with(".") || source.starts_with("..");
      if !is_normal_import {
        // TODO: resolve external module
        // ignore all external module for now
        return None;
      }
      let importer = importer?;
      let importer_dir = Path::new(importer).parent()?;
      nodejs::join(importer_dir, &source)
    };

    id.set_extension("js");
    id.to_str().map(|p| p.to_owned())
  }

  #[inline]
  pub fn load(&self, id: &str) -> Option<String> {
    // TODO: call hook load of plugins
    None
  }

  pub fn transform(&self, code: String, id: &str) -> String {
    code
  }

  pub fn module_parsed(&self) {}

  pub fn resolve_dynamic_import(&self) {}

  pub fn build_end(&self) {}
}
