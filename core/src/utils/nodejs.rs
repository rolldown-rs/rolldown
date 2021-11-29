use std::{
  env,
  path::{Component, Path, PathBuf},
};

use once_cell::sync::Lazy;

pub(crate) static CURRENT_DIR: Lazy<String> =
  Lazy::new(|| env::current_dir().unwrap().to_str().unwrap().to_owned());

// https://www.reddit.com/r/rust/comments/hkkquy/anyone_knows_how_to_fscanonicalize_but_without/
#[inline]
fn normalize_path(path: &Path) -> PathBuf {
  let mut components = path.components().peekable();
  let mut need_next = false;
  let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek() {
    need_next = true;
    PathBuf::from(c.as_os_str())
  } else {
    PathBuf::new()
  };
  if need_next {
    components.next();
  }
  components.for_each(|component| match component {
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
  });
  ret
}

#[inline]
pub fn resolve(path: &Path) -> PathBuf {
  let p = Path::new(CURRENT_DIR.as_str()).join(path);
  normalize_path(&p)
}

#[inline]
pub fn join(p1: &Path, p2: &Path) -> PathBuf {
  let p = Path::new(p1).join(p2);
  normalize_path(&p)
}

pub fn relative(p1: &Path, p2: &Path) -> PathBuf {
  // FIXME: implement is not good enough
  let p2 = p2.strip_prefix(p1).unwrap().to_owned();

  ("./".to_owned() + &p2.to_str().unwrap()).into()
}
