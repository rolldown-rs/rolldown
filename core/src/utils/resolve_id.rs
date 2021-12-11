use std::path::Path;

use crate::types::{ResolveIdResult, ResolvedId};

use super::{plugin_driver::PluginDriver};

fn is_absolute(path: &str) -> bool {
  Path::new(path).is_absolute()
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
  source: &str,
  importer: Option<&str>,
  plugin_driver: &PluginDriver,
) -> Option<ResolvedId> {
  plugin_driver.resolve_id(source, importer)
}

fn default_resolve_id(source: &str, importer: Option<&str>, _preserve_symlinks: bool) -> String {
  let mut id = if nodejs_path::is_absolute(source) {
    source.to_owned()
  } else if importer.is_none() {
    nodejs_path::resolve!(&source)
  } else {
    let importer = importer.unwrap();
    let importer_dir = nodejs_path::dirname(&importer);
    nodejs_path::join!(&importer_dir, &source)
  };

  add_js_extension_if_necessary(id, false)
}


fn add_js_extension_if_necessary(mut file: String, preserveSymlinks: bool) -> String {
  // FIXME: The implement isn't right.
  if nodejs_path::extname(&file) != ".js" {
    file.push_str(".js");
  }
  file
	// let found = findFile(file, preserveSymlinks);
	// if (found) return found;
	// found = findFile(file + '.mjs', preserveSymlinks);
	// if (found) return found;
	// found = findFile(file + '.js', preserveSymlinks);
	// return found;
}

// fn findFile(file: &str, preserveSymlinks: bool) -> bool {
// 	try {
// 		const stats = lstatSync(file);
// 		if (!preserveSymlinks && stats.isSymbolicLink())
// 			return findFile(realpathSync(file), preserveSymlinks);
// 		if ((preserveSymlinks && stats.isSymbolicLink()) || stats.isFile()) {
// 			// check case
// 			const name = basename(file);
// 			const files = readdirSync(dirname(file));

// 			if (files.indexOf(name) !== -1) return file;
// 		}
// 	} catch {
// 		// suppress
// 	}
// }
