use std::path::Path;

use crate::utils::nodejs;

pub fn resolve_id(source: &str, importer: Option<&str>) -> Option<String> {
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

#[cfg(test)]
#[cfg(not(target_os = "windows"))]
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
