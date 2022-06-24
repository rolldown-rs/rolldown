use std::collections::HashMap;

use crate::{DevServerOptions, EntryItem};

#[derive(Debug, Default)]
pub struct CompilerOptions {
  pub entries: HashMap<String, EntryItem>,
  pub root: String,
  pub dev_server: DevServerOptions,
}
