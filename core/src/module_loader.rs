use std::collections::HashMap;

use once_cell::sync::Lazy;
use swc_common::{sync::Lrc, SourceMap};

use crate::utils::resolve_id::{PartialId, resolve_id};
use crate::utils::transform::transform;
use crate::{external_module::ExternalModule, module::Module};
use crate::{
  types::{shared, Shared},
  utils::plugin_driver::PluginDriver,
  GraphError,
};
use crate::{utils, ModOrExt};

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
    let source = self
      .plugin_driver
      .borrow()
      .load(id)
      .unwrap_or(std::fs::read_to_string(id).unwrap());
    // hook `load` was called

    module.update_options();

    let transformed = transform(source, module, &self.plugin_driver.borrow());
    // hook `transform` was called

    module.set_source(transformed)
  }

  pub(crate) fn fetch_module(
    &mut self,
    resolved_id: ResolvedId,
    importer: Option<&str>,
    is_entry: bool,
  ) -> Shared<Module> {
    let id = &resolved_id.id;
    if let Some(ModOrExt::Mod(m)) = self.modules_by_id.get(id) {
      m.clone()
    } else {
      let module = Module::new(id.into(), is_entry);
      self.modules_by_id.insert(id.into(), module.clone().into());
      self.add_module_source(id, importer, &mut module.borrow_mut());
      self.get_resolve_static_dependency(&module.borrow());
      module
    }

  }

  fn get_resolve_static_dependency(&self, module: &Module) {

  }

  // pub fn get_entry_module(&self) -> Shared<Module> {
  //   let entry_module = self
  //     .fetch_module(&self.entry, None)
  //     .unwrap()
  //     .into_mod()
  //     .expect("entry module not found");

  //   entry_module
  // }

  pub fn add_entry_modules(
    &mut self,
    entries: &[UnresolvedModule],
    _is_user_defined: bool,
  ) -> Vec<Shared<Module>> {
    let entry_modules = entries
      .iter()
      .map(|unresolved| {
        self.load_entry_module(
          &unresolved.id,
          true,
          unresolved.importer.as_ref().map(|s| s.as_str()),
        )
      })
      .collect::<Vec<Shared<Module>>>();

    entry_modules.iter().for_each(|entry_module| {
      // entry_module.borrow_mut().is
    });

    entry_modules
  }

  pub fn load_entry_module(
    &mut self,
    unresolved_id: &str,
    is_entry: bool,
    importer: Option<&str>,
  ) -> Shared<Module> {
    let resolve_id_result = resolve_id(unresolved_id, importer, false, &self.plugin_driver.borrow());
    // hook `resoveId` was called

    if let Some(resolve_id_result) = resolve_id_result {

      self.fetch_module(self.add_defaults_to_resolved_id(resolve_id_result), importer, is_entry)
    } else {
      panic!("resolve_id_result is None")
    }
  }

  fn add_defaults_to_resolved_id(&self, part: PartialId) -> ResolvedId {

    ResolvedId {
      external: part.external,
      id: part.id,
      module_side_effects: true,
    }
  }

  // fn get_resolve_static_dependency_promises(module: &Module) -> Vec<(String, ResolvedId)> {
  //   module.sources.iter().map(|source| {
  //     let resolved_id;
  //     if let Some(resolved) = module.resolved_ids.get(source) {
  //       resolved_id = resolved.clone()
  //     } else {
  //       resolved_id 
  //     };
  //     (source.clone(), resolved_id)
  //   }).collect()
  // }

  // fn resolve_id() -> Option<ResolvedId> {
  //   reso
  // }
}


pub struct ResolvedId {
  pub id: String,
  pub external: bool,
  pub module_side_effects: bool,
}

pub struct UnresolvedModule {
  pub file_name: Option<String>,
  pub id: String,
  pub importer: Option<String>,
  pub name: Option<String>,
}
