use log::debug;
use path_absolutize::*;
use std::collections::HashMap;
use std::env;
use std::io;
use std::mem;
use std::path::PathBuf;
use std::path::{Path, MAIN_SEPARATOR};

use ahash::RandomState;
use once_cell::sync::Lazy;
use swc_common::sync::RwLock;
mod built_in;

#[derive(Clone)]
#[non_exhaustive]
pub struct HookDriver;

impl Default for HookDriver {
  fn default() -> Self {
    Self::new()
  }
}

impl HookDriver {
  pub fn new() -> Self {
    HookDriver
  }

  // build hooks
  pub fn options() {}

  pub fn build_start() {}

  pub fn resolve_id(
    &self,
    source: &str,
    importer: Option<&str>,
    _parent_dir_cache: &RwLock<HashMap<String, String, RandomState>>,
  ) -> Option<String> {
    let id = built_in::resolve_id(source, importer);
    debug!("resolve_id: {:?}", id);
    id
  }

  pub fn load(&self, id: &str) -> io::Result<String> {
    debug!("load id: {}", id);
    std::fs::read_to_string(id)
  }

  pub fn transform() {}

  pub fn module_parsed() {}

  pub fn resolve_dynamic_import() {}

  pub fn build_end() {}
}
