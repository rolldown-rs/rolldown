use crate::Module;

use super::plugin_driver::{self, PluginDriver};

pub fn transform(source: String, module: &Module, plugin_driver: &PluginDriver) -> String {
  let id = &module.id;

  plugin_driver.transform(source, id)
}
