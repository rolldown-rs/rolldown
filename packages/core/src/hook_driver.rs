use std::env;
use std::io;
use std::path::{Path, PathBuf};

fn resolve(path: &str) -> io::Result<PathBuf> {
  Ok(Path::join(env::current_dir()?.as_path(), path))
}

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

  pub fn resolve_id(&self, source: &str, importer: Option<&str>) -> Option<String> {
    if Path::new(source).is_absolute() {
      return Some(source.to_owned());
    };

    if importer.is_none() {
      return resolve(source).ok()?.to_str().map(|p| p.to_owned());
    }

    if !source.starts_with('.') {
      // TODO: resolve external module
      // ignore all external module for now
      return None;
    }

    let importer_dir = Path::new(importer?).parent()?;
    let mut result = importer_dir.join(source);
    result.set_extension("js");
    result.to_str().map(|p| p.to_owned())
  }
  pub fn load(&self, id: &str) -> io::Result<String> {
    std::fs::read_to_string(id)
  }

  pub fn transform() {}

  pub fn module_parsed() {}

  pub fn resolve_dynamic_import() {}

  pub fn build_end() {}
}
