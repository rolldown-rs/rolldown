use std::path::Path;

use super::{nodejs, plugin_driver::PluginDriver};

fn is_absolute(path: &str) -> bool {
  return false;
}

pub struct PartialId {
  pub external: bool,
  pub id: String,
}

type ResolveIdResult = PartialId;

pub fn resolve_id(
  source: &str,
  importer: Option<&str>,
  preserve_symlinks: bool,
  plugin_driver: &PluginDriver,
) -> Option<ResolveIdResult> {
  let plugin_result = resolve_id_via_plugins(source, importer, plugin_driver);
  if plugin_result.is_some() {
    plugin_result
  } else {
    // external modules (non-entry modules that start with neither '.' or '/')
    // are skipped at this stage.
    if importer.is_none() && !is_absolute(source) && !source.starts_with(".") {
      None
    } else {
      Some(PartialId {
        external: false,
        id: default_resolve_id(source, importer, preserve_symlinks),
      })
    }
  }
}

pub fn resolve_id_via_plugins(
  source: &str,
  importer: Option<&str>,
  plugin_driver: &PluginDriver,
) -> Option<ResolveIdResult> {
  // TODO: call hook resolveId of plugins
  None
}

fn default_resolve_id(
  source: &str,
  importer: Option<&str>,
  _preserveSymlinks: bool,
) -> String {
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
