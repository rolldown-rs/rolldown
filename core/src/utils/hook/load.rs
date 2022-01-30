use crate::plugin_driver::PluginDriver;

pub fn load(id: &str) -> String {
  log::debug!("load {}", id);
  std::fs::read_to_string(id).unwrap()
}
