use std::path::Path;

use crate::types::{Shared, shared};

use super::nodejs;


pub struct PluginDriver {

}

impl PluginDriver {
  pub fn new() -> Shared<Self> {
    shared(Self {})
  }
}

impl PluginDriver {
  // build hooks
  pub fn options() {}

  pub fn build_start() {}

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
  pub fn load(&self, id: &str) -> std::io::Result<String> {
    // debug!("load id: {}", id);
    std::fs::read_to_string(id)
  }

  pub fn transform(&self, code: String) -> String {
    code
  }

  pub fn module_parsed(&self, ) {}

  pub fn resolve_dynamic_import(&self) {}

  pub fn build_end(&self) {}
}
