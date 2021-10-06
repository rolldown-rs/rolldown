use std::{
  env,
  path::{Component, Path, PathBuf},
};

use once_cell::sync::Lazy;

static CURRENT_DIR: Lazy<String> =
  Lazy::new(|| env::current_dir().unwrap().to_str().unwrap().to_owned());

// https://www.reddit.com/r/rust/comments/hkkquy/anyone_knows_how_to_fscanonicalize_but_without/
fn normalize_path(path: &Path) -> PathBuf {
  let mut components = path.components().peekable();
  let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
    components.next();
    PathBuf::from(c.as_os_str())
  } else {
    PathBuf::new()
  };

  for component in components {
    match component {
      Component::Prefix(..) => unreachable!(),
      Component::RootDir => {
        ret.push(component.as_os_str());
      }
      Component::CurDir => {}
      Component::ParentDir => {
        ret.pop();
      }
      Component::Normal(c) => {
        ret.push(c);
      }
    }
  }
  ret
}

pub mod path {
  use super::*;
  pub fn resolve(path: &Path) -> PathBuf {
    // let mut result = String::with_capacity(CURRENT_DIR.len() + path.len() + 1);
    // result.push_str(CURRENT_DIR.as_str());
    // result.push(MAIN_SEPARATOR);
    // result.push_str(path);
    // result
    let p = Path::new(CURRENT_DIR.as_str()).join(path);
    normalize_path(&p)
  }

  pub fn join(p1: &Path, p2: &Path) -> PathBuf {
    let p = Path::new(p1).join(p2);
    normalize_path(&p)
  }
}
