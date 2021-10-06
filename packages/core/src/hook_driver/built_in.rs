use std::{
  path::{Path},
};

use crate::utils::nodejs;
pub fn resolve_id(
  source: &str,
  importer: Option<&str>,
  // _parent_dir_cache: &RwLock<HashMap<String, String, RandomState>>,
) -> Option<String> {
  

  let source = Path::new(source).to_path_buf();
  let mut id;
  if source.is_absolute() {
    id = source
  } else if importer.is_none() {
    id = nodejs::path::resolve(&source);
  } else {
    if !source.starts_with(".") {
      // TODO: resolve external module
      // ignore all external module for now
      return None;
    }
    let importer = importer?;
    let importer_dir = Path::new(importer).parent()?;
    id = nodejs::path::join(importer_dir, &source);
  }

  id.set_extension("js");
  id.to_str().map(|p| p.to_owned())
  // let read_lock = parent_dir_cache.read();
  // if let Some(parent) = read_lock.get(importer) {
  //   // let importer_dir = Path::new(parent);
  //   let mut result = node_join(parent, source);
  //   mem::drop(read_lock);
  //   result.set_extension("js");
  //   result.to_str().map(|p| p.to_owned())
  // } else {
  //   mem::drop(read_lock);
  //   let importer_dir = Path::new(importer).parent()?;
  //   let mut write_cache = parent_dir_cache.write();
  //   write_cache.insert(
  //     importer.to_owned(),
  //     importer_dir.to_str().unwrap().to_owned(),
  //   );
  //   mem::drop(write_cache);
  //   let mut result = node_join(importer, source);
  //   result.set_extension("js");
  //   result.to_str().map(|p| p.to_owned())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn absolute() {
    let left = resolve_id("/foo/bar/index", None);
    let right = "/foo/bar/index.js";
    assert_eq!(left, Some(right.to_owned()));
  }


  #[test]
  fn relative_contains_dot() {
    let left = resolve_id(".././baz", Some("/foo/bar/index.js"));
    let right = "/foo/baz.js";
    assert_eq!(left, Some(right.to_owned()));
  }
}
