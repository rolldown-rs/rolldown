use std::path::Path;

use crate::types::ResolvedId;

use super::plugin_driver::PluginDriver;

fn is_absolute(path: &str) -> bool {
  Path::new(path).is_absolute()
}

pub fn resolve_id(
  source: &str,
  importer: Option<&str>,
  preserve_symlinks: bool,
  // plugin_driver: &PluginDriver,
) -> ResolvedId {
  let res = if importer.is_some() && !is_absolute(source) && !source.starts_with(".") {
    ResolvedId::new(source.to_owned(), true)
  } else {
    ResolvedId::new(
      default_resolve_id(source, importer, preserve_symlinks),
      false,
    )
  };
  // debug!(
  //   "resolve {} with importer {:?} got {:?}",
  //   source, importer, res
  // );
  res
}

pub fn resolve_id_via_plugins(
  source: &str,
  importer: Option<&str>,
  plugin_driver: &PluginDriver,
) -> Option<ResolvedId> {
  plugin_driver.resolve_id(source, importer)
}

fn default_resolve_id(source: &str, importer: Option<&str>, _preserve_symlinks: bool) -> String {
  let id = if nodejs_path::is_absolute(source) {
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

// FIXME: the implement is not align to Rollup now.
fn add_js_extension_if_necessary(mut file: String, _preserve_symlinks: bool) -> String {
  // FIXME: The implement isn't right. The correct implement is below there.
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
