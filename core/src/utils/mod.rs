pub mod nodejs;
pub mod plugin_driver;
pub mod resolve_id;
pub mod transform;

pub mod path {
  use std::path::{Path, PathBuf};

  use super::nodejs::{self, relative};

  pub fn relative_id(id: PathBuf) -> PathBuf {
    if id.is_absolute() {
      relative(&nodejs::resolve(Path::new(".")), &id)
    } else {
      id
    }
  }
}
