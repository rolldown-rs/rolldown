use crate::module::{Module, ModuleOptions};
use path_absolutize::Absolutize;
use std::{cell::RefCell, collections::HashMap, fs, path::Path, rc::Rc};

#[derive(Debug)]
pub struct Bundle {
  pub entry_path: String,
  pub base: String,
  pub entry_module: RefCell<Option<Rc<Module>>>,
  pub modules: RefCell<HashMap<String, Rc<Module>>>,
  // pub statements: Vec<String>,
  // pub external_modules: Vec<String>,
  // pub internal_namespace_modules: Vec<String>,
}
impl Bundle {
  pub fn new(entry: &str) -> Rc<Bundle> {
    let mut p = Path::new(entry).to_path_buf();
    p.set_extension("js");
    let entry_path = p.absolutize().unwrap().to_str().unwrap().to_owned();
    let base = p
      .parent()
      .unwrap()
      .absolutize()
      .unwrap()
      .to_str()
      .unwrap()
      .to_owned();
    Rc::new(Bundle {
      entry_path,
      base,
      modules: RefCell::new(HashMap::new()),
      entry_module: RefCell::new(None),
    })
  }

  fn build(self: &Rc<Self>) {
    let module = self.fetch_module(&self.entry_path.clone(), None);
    self.entry_module.borrow_mut().replace(module);
  }
  fn fetch_module(self: &Rc<Self>, importee: &str, maybe_importer: Option<&str>) -> Rc<Module> {
    let mut modules = self.modules.borrow_mut();

    if let Some(m) = modules.get(importee) {
      m.clone()
    } else {
      let route;
      if let Some(importer) = maybe_importer {
        let importee_path = Path::new(importee);
        if importee_path.is_absolute() {
          route = importee_path.to_str().unwrap().to_owned();
        } else if importee_path.starts_with(".") {
          let importer_dir = Path::new(importer).parent().unwrap();
          route = importer_dir.join(importee).to_str().unwrap().to_owned();
        } else {
          // for bare import
          route = "".to_owned()
        }
      } else {
        route = importee.to_owned();
      }
      if !route.eq("") {
        let code = fs::read_to_string(&route).unwrap();
        let m = Module::new(ModuleOptions {
          code,
          bundle: self.clone(),
          path: route.clone(),
        });
        modules.insert(route.clone(), m.clone());
        m
      } else {
        todo!("external module")
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::Bundle;
  use std::{env, path::Path};

  #[test]
  fn extensiton_test() {
    let b = Bundle::new("./foo/bar");
    assert!(b.entry_path.ends_with(".js"));
  }
  #[test]
  fn entry_path_test() {
    let b = Bundle::new("./foo/bar");
    let left = Path::new(&b.entry_path).to_path_buf();
    let right = Path::join(env::current_dir().unwrap().as_path(), "./foo/bar.js");
    assert_eq!(left, right);
  }
  #[test]
  fn base_test() {
    let b = Bundle::new("./foo/bar");
    let right = Path::new(&b.base).to_path_buf();
    let left = Path::join(env::current_dir().unwrap().as_path(), "./foo");
    assert_eq!(left, right);
  }

  #[test]
  fn e2e() {
    let b = Bundle::new("./demo/main");
    b.build();
    let m = b.entry_module.borrow();
    let t = &m.as_ref().unwrap().imports;
    // println!("imports {:?}", t);
  }
}
