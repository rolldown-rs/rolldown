use std::collections::{HashMap};

use once_cell::sync::Lazy;
use swc_common::{
  sync::{Lrc},
  SourceMap,
};

use crate::{GraphError, types::{shared, Shared}, utils::plugin_driver::PluginDriver};
use crate::{ModOrExt};
use crate::{external_module::ExternalModule, module::Module};

pub(crate) static SOURCE_MAP: Lazy<Lrc<SourceMap>> = Lazy::new(Default::default);


#[derive(Clone)]
pub struct ModuleLoader {
  // cached module
  entry: String,
  modules_by_id: HashMap<String, ModOrExt>,
  plugin_driver: Shared<PluginDriver>,
}

impl ModuleLoader {
  pub fn new(entry: String, plugin_driver: Shared<PluginDriver>) -> Shared<Self> {
    shared(Self {
      entry,
      modules_by_id: HashMap::default(),
      plugin_driver,
    })
  }

  #[inline]
  pub fn get_module(&self, id: &str) -> Option<ModOrExt> {
    self.modules_by_id.get(id).cloned()
  }

  #[inline]
  pub(crate) fn insert_module(&mut self, id: String, module: ModOrExt) {
    self.modules_by_id.insert(id, module);
  }

  fn add_module_source(&self, id: &str, importer: Option<&str>, module: &mut Module) {
    let source = self.plugin_driver.borrow().load(id).unwrap();
    module.original_code = Some(source)

  }

  pub(crate) fn fetch_module(
    &mut self,
    id: &str,
    importer: Option<&str>,
    is_entry: bool
  ) -> Shared<Module> {
    let module = Module::new(id.into(), is_entry);

    self.modules_by_id.insert(id.into(), module.clone().into());

    self.add_module_source(id, importer, &mut module.borrow_mut());


    module

    // let plugin_driver = self.plugin_driver.borrow();
    // plugin_driver
    //   .resolve_id(source, importer)
    //   .map(|id| {
    //     self.get_module(&id).map(Ok).unwrap_or_else(|| {
    //       let source = plugin_driver.load(&id).unwrap();
    //       let transformed = plugin_driver.transform(source);
    //       if let Ok(module) = Module::new(source, id.clone()) {
    //         plugin_driver.module_parsed();
    //         self.insert_module(id, module.clone().into());
    //         Ok(module.into())
    //       } else {
    //         Err(GraphError::ParseModuleError)
    //       }
    //     })
    //   })
    //   .unwrap_or_else(|| {
    //     self.get_module(source).map(Ok).unwrap_or_else(|| {
    //       let module = ExternalModule::new(source.to_owned());
    //       self.insert_module(source.to_owned(), module.clone().into());
    //       Ok(module.into())
    //     })
    //   })
  }

  // pub fn get_entry_module(&self) -> Shared<Module> {
  //   let entry_module = self
  //     .fetch_module(&self.entry, None)
  //     .unwrap()
  //     .into_mod()
  //     .expect("entry module not found");

  //   entry_module
  // }


  pub fn add_entry_modules(&mut self, entry: &str, is_user_defined: bool) -> Shared<Module> {
    return self.load_entry_module(entry, None);
  }

  pub fn load_entry_module(&mut self, unresolved_id: &str, importer: Option<&str>) -> Shared<Module> {
    let resolved_id = self.plugin_driver.borrow().resolve_id(unresolved_id, importer);
    self.fetch_module(&resolved_id.unwrap(), importer, true)
  }
}