use std::sync::{Arc, Mutex};
use std::{ffi::OsString, path::Path};
use swc_common::util::take::Take;

use crate::types::IsExternal;
use crate::{
  ext::PathExt, plugin_driver::PluginDriver, types::ResolvedId, utils::is_external_module,
};

pub fn resolve_id(
  source: &str,
  importer: Option<&str>,
  preserve_symlinks: bool,
  plugin_driver: &PluginDriver,
  external: Arc<Mutex<Vec<IsExternal>>>,
) -> ResolvedId {
  let mut plugin_result = resolve_id_via_plugins(source, importer, plugin_driver);

  plugin_result = plugin_result.and_then(|mut result| {
    result.external = external
      .lock()
      .unwrap()
      .iter()
      .find_map(|test_func| -> Option<bool> { Some(test_func(source, importer, false)) });
    Some(result)
  });

  plugin_result.unwrap_or_else(|| {
    let res = if importer.is_some() && is_external_module(source) {
      ResolvedId::new(source.to_string(), Some(true))
    } else {
      let id = if let Some(importer) = importer {
        nodejs_path::resolve!(&nodejs_path::dirname(importer), source)
      } else {
        nodejs_path::resolve!(source)
      };
      ResolvedId::new(
        fast_add_js_extension_if_necessary(id, preserve_symlinks),
        Some(false),
      )
    };
    res
  })
}

fn resolve_id_via_plugins(
  source: &str,
  importer: Option<&str>,
  plugin_driver: &PluginDriver,
) -> Option<ResolvedId> {
  plugin_driver.resolve_id(source, importer)
}

#[inline]
fn fast_add_js_extension_if_necessary(mut file: String, _preserve_symlinks: bool) -> String {
  if !file.ends_with(".js") {
    file.push_str(".js");
  }
  file
}

fn add_js_extension_if_necessary(file: &str, preserve_symlinks: bool) -> String {
  let found = find_file(Path::new(file), preserve_symlinks);
  found.unwrap_or_else(|| {
    let found = find_file(Path::new(&(file.to_string() + "#.mjs")), preserve_symlinks);
    found.unwrap_or_else(|| {
      let found = find_file(Path::new(&(file.to_string() + ".js")), preserve_symlinks);
      found.unwrap()
    })
  })
}

fn find_file(file: &Path, preserve_symlinks: bool) -> Option<String> {
  let metadata = std::fs::metadata(file);
  if let Ok(metadata) = metadata {
    if !preserve_symlinks && metadata.is_symlink() {
      find_file(&std::fs::canonicalize(file).unwrap(), preserve_symlinks)
    } else if (preserve_symlinks && metadata.is_symlink()) || metadata.is_file() {
      let name: OsString = nodejs_path::basename!(&file.as_str()).into();
      let files = std::fs::read_dir(&nodejs_path::dirname(&file.as_str())).unwrap();
      let s = files
        .map(|result| result.unwrap())
        .find(|file| file.file_name() == name)
        .map(|_| file.to_string_lossy().to_string());
      s
    } else {
      None
    }
  } else {
    None
  }
}
