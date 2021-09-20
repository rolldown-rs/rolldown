use crate::{external_module::ExternalModule, module::{Module, ModuleOptions}, new_type::shared::Shared, statement::Statement};
use path_absolutize::Absolutize;
use std::{borrow::BorrowMut, cell::RefCell, collections::HashMap, env, fs, path::{Path, PathBuf}, rc::Rc, vec};

fn resolve(path: &str) -> PathBuf {
  Path::join(env::current_dir().unwrap().as_path(), path)
}

#[derive(Debug, Clone)]
pub struct Bundle {
  pub entry: String,
  pub entry_module: Option<Shared<Module>>,
  pub var_exports: HashMap<String, String>,
  pub to_export: Option<String>,
  pub modules: HashMap<String, Shared<Module>>,
  pub statements: Vec<Statement>,
  pub external_modules: Vec<ExternalModule>,
  pub internal_namespace_modules: Vec<String>,
  pub assumed_globals: HashMap<String, String>,
}
impl Bundle {
  fn resolve_id<'a, 'b>(&self, importee: &'a str, importer: Option<&'b str>) -> Option<String> {
    if Path::new(importee).is_absolute() { return  Some(importee.to_owned()) };

    if importer.is_none() { return Some(resolve(importee).to_str().unwrap().to_owned()) }

    if !importee.starts_with(".") {
      // TODO: resolve external module
      // ignore all external module for now
      return None
    }

    return {
      let importer_dir = Path::new(importer.unwrap()).parent().unwrap();
      let mut result = importer_dir.join(importee);
      result.set_extension("js");
      Some(result.to_str().unwrap().to_owned())
    }
  }

  fn load(&self, id: &str) -> String {
    std::fs::read_to_string(id).unwrap()
  }
}
impl Bundle {
  pub fn new(entry: &str) -> Bundle {
    // let mut p = Path::new(entry).to_path_buf();
    // p.set_extension("js");
    // let entry_path = p.absolutize().unwrap().to_str().unwrap().to_owned();
    // let base = p
    //   .parent()
    //   .unwrap()
    //   .absolutize()
    //   .unwrap()
    //   .to_str()
    //   .unwrap()
    //   .to_owned();
    Bundle {
      entry: entry.to_owned(),
      entry_module: None,
      modules: HashMap::new(),
      var_exports: HashMap::new(),
      to_export: None,
      statements: vec![],
      external_modules: vec![],
      internal_namespace_modules: vec![],
      assumed_globals: HashMap::new(),
    }
  }

  pub fn build(this: &Shared<Bundle>) {
    let entry_module = Bundle::fetch_module(this, &this.entry, None);
    // this.borrow_mut().entry_module.replace(module);
  }
  fn fetch_module(this: &Shared<Self>, importee: &str, importer: Option<&str>) -> Shared<Module> {
    let id = this.resolve_id(importee, importer);

    if let Some(id) = id {
      if !this.modules.contains_key(&id) {
        let source = this.load(&id);
        let module = Module::new(ModuleOptions {
          source,
          id: id.to_string(),
          bundle: this.clone(),
        });
        // this
        //   .modules
        //   .borrow_mut()
        //   .insert(k, v)
      }
    } else {
      // external module
      todo!("external module")
    }
  

    let modules = &mut this.borrow_mut().modules;

    if let Some(m) = modules.get(importee) {
      m.clone()
    } else {
      let route;
      if let Some(importer) = importer {
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
        let source = fs::read_to_string(&route).unwrap();
        let m = Module::new(ModuleOptions {
          source,
          bundle: this.clone(),
          id: route.clone(),
        });
        let m = m;
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
    assert!(b.entry.ends_with(".js"));
  }
  #[test]
  fn entry_path_test() {
    let b = Bundle::new("./foo/bar");
    let left = Path::new(&b.entry).to_path_buf();
    let right = Path::join(env::current_dir().unwrap().as_path(), "./foo/bar.js");
    assert_eq!(left, right);
  }

  #[test]
  fn e2e() {
    let b = Bundle::new("./demo/main");

    // b.build();
    // let m = b.entry_module.borrow();
    // let t = &m.as_ref().unwrap().imports;
    // println!("imports {:?}", t);
  }
}
