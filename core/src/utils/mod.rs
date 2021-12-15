pub mod chunk_assignment;
pub mod plugin_driver;
pub mod resolve_id;
pub mod transform;
pub mod ast;

pub mod path {
  pub fn relative_id(id: String) -> String {
    if nodejs_path::is_absolute(&id) {
      nodejs_path::relative(&nodejs_path::resolve!("."), &id)
    } else {
      id
    }
  }
}
