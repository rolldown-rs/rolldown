use std::path::Path;

use crate::{module_loader::ResolvedId, types::ResolveIdResult};

use super::{nodejs, plugin_driver::PluginDriver};

fn is_absolute(_path: &str) -> bool {
  return false;
}

pub fn resolve_id(
  source: &str,
  importer: Option<&str>,
  preserve_symlinks: bool,
  plugin_driver: &PluginDriver,
) -> ResolveIdResult {
  let plugin_result = resolve_id_via_plugins(source, importer, plugin_driver);
  if plugin_result.is_some() {
    plugin_result
  } else {
    // external modules (non-entry modules that start with neither '.' or '/')
    // are skipped at this stage.
    if !importer.is_none() && !is_absolute(source) && !source.starts_with(".") {
      None
    } else {
      Some(ResolvedId::new(
        default_resolve_id(source, importer, preserve_symlinks),
        false,
      ))
    }
  }
}

pub fn resolve_id_via_plugins(
  _source: &str,
  _importer: Option<&str>,
  _plugin_driver: &PluginDriver,
) -> Option<ResolvedId> {
  // TODO: call hook resolveId of plugins
  None
}

fn default_resolve_id(source: &str, importer: Option<&str>, preserve_symlinks: bool) -> String {
  let source = Path::new(source).to_path_buf();
  let mut id = if source.is_absolute() {
    source
  } else if importer.is_none() {
    nodejs::resolve(&source)
  } else {
    let importer = importer.unwrap();
    let importer_dir = Path::new(importer).parent().unwrap();
    nodejs::join(importer_dir, &source)
  };

  id.set_extension("js");
  id.to_str().map(|p| p.to_owned()).unwrap()
}
