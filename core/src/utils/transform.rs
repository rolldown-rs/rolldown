use crate::Module;

use super::plugin_driver::PluginDriver;

pub fn transform(source: String, module: &Module, plugin_driver: &PluginDriver) -> String {
  let id = &module.id;

  plugin_driver
    .transform(source.clone(), id)
    .unwrap_or(source)
}
