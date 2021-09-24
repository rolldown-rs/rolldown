use crate::{
  external_module::ExternalModule,
  graph,
  module::{Module, ModuleOptions},
  statement::Statement,
  types::shared::Shared,
};
use path_absolutize::Absolutize;
use std::{
  borrow::BorrowMut,
  cell::RefCell,
  collections::HashMap,
  env, fs,
  path::{Path, PathBuf},
  rc::Rc,
  vec,
};
use swc_common::{BytePos, Span, SyntaxContext, DUMMY_SP};

#[derive(Clone)]
pub struct Bundle {
  pub graph: graph::Graph,
  // pub entry_module: Option<Shared<Module>>,
  // pub modules: HashMap<String, Shared<RollDownModule>>,
  // pub statements: Vec<Statement<'a>>,
  // pub external_modules: Vec<ExternalModule>,
  // pub internal_namespace_modules: Vec<String>,
}
impl Bundle {
  fn new() {}
}

#[cfg(test)]
mod tests {
  use crate::types::shared::Shared;

  use super::Bundle;
  use std::{env, path::Path};

  // #[test]
  // fn extensiton_test() {
  //   let b = Bundle::new("./foo/bar");
  //   assert!(b.entry.ends_with(".js"));
  // }
  // #[test]
  // fn entry_path_test() {
  //   let b = Bundle::new("./foo/bar");
  //   let left = Path::new(&b.entry).to_path_buf();
  //   let right = Path::join(env::current_dir().unwrap().as_path(), "./foo/bar.js");
  //   assert_eq!(left, right);
  // }

  #[test]
  fn e2e() {
    // let b = Bundle::new("demo/main.js");
    // Bundle::build(&Shared::new(b));
    // let m = b.entry_module.borrow();
    // let t = &m.as_ref().unwrap().imports;
    // println!("imports {:?}", t);
  }
}
