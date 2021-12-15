use crate::{types::ResolvedId, utils::plugin_driver::Plugin};

pub struct Alias {
  entries: Entries
}

type Entries = Vec<(Find, Replacement)>;
type Find = String;
type Replacement = String;

/// # Example
/// ```no_run
/// Rolldown::plugin::alias(vec![("react".to_owned(), "preact".to_owned())])
/// ```
pub fn new(entries: Entries) -> Alias {
  Alias {
    entries,
  }
}

impl Plugin for Alias {
  fn get_name(&self) -> &'static str {
      "rusty-alias"
  }

  fn resolve_id(&self, importee: &str, _importer: Option<&str>) -> crate::types::ResolveIdResult {
      let matched_entry = self.entries.iter().find(|(find, _)| importee.contains(find));
      
      matched_entry.map(|(find, replacement)| {
        let replaced = importee.to_owned().replace(find, &replacement);
        
        ResolvedId {
          external: !replaced.starts_with(".") && !replaced.starts_with(".."),
          id: replaced,
        }
      })
  }

}

