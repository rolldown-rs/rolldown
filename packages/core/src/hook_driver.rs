use std::{
  env,
  path::{Path, PathBuf},
};

fn resolve(path: &str) -> PathBuf {
  Path::join(env::current_dir().unwrap().as_path(), path)
}

#[derive(Clone)]
pub struct HookDriver {}

impl HookDriver {
  // build hooks
  pub fn options() {}
  pub fn build_start() {}
  pub fn resolve_id(&self, source: &str, importer: Option<&str>) -> Option<String> {
    if Path::new(source).is_absolute() {
      return Some(source.to_owned());
    };

    if importer.is_none() {
      return Some(resolve(source).to_str().unwrap().to_owned());
    }

    if !source.starts_with(".") {
      // TODO: resolve external module
      // ignore all external module for now
      return None;
    }

    return {
      let importer_dir = Path::new(importer.unwrap()).parent().unwrap();
      let mut result = importer_dir.join(source);
      result.set_extension("js");
      Some(result.to_str().unwrap().to_owned())
    };
  }
  pub fn load(&self, id: &str) -> String {
    std::fs::read_to_string(id).unwrap()
  }
  pub fn transform() {}
  pub fn module_parsed() {}
  pub fn resolve_dynamic_import() {}
  pub fn build_end() {}
}
