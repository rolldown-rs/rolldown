use crate::types::{shared, NormalizedInputOptions, Shared};
use crate::utils::execution_order::analyse_module_execution;
use crate::utils::plugin_driver::PluginDriver;
use crate::{external_module::ExternalModule, module::Module};
use crate::{
  module_loader::ModuleLoader,
  types::{ModOrExt, UnresolvedModule},
};

#[derive(Clone)]
#[non_exhaustive]
pub struct Graph {
  pub options: Shared<NormalizedInputOptions>,
  pub entry_modules: Vec<Shared<Module>>,
  pub module_loader: Shared<ModuleLoader>,
  pub plugin_driver: Shared<PluginDriver>,
  pub modules: Vec<Shared<Module>>,
  pub external_modules: Vec<Shared<ExternalModule>>,
}

impl Graph {
  pub fn new(options: NormalizedInputOptions) -> Self {
    env_logger::init();

    let options = shared(options);

    let plugin_driver = PluginDriver::new(options.clone());
    let module_container = ModuleLoader::new(plugin_driver.clone());

    let graph = Self {
      options,
      entry_modules: vec![],
      module_loader: module_container,
      plugin_driver,
      modules: vec![],
      external_modules: vec![],
    };

    graph
  }

  // build dependency graph via entry modules.
  pub fn generate_module_graph(&mut self) {

    self.entry_modules = self.module_loader.borrow_mut().add_entry_modules(
      &normalize_entry_modules(self.options.borrow().input.clone()),
      true,
    );

    self
      .module_loader
      .borrow()
      .modules_by_id
      .values()
      .for_each(|mod_or_ext| match mod_or_ext {
        ModOrExt::Ext(module) => {
          self.external_modules.push(module.clone());
        }
        ModOrExt::Mod(module) => {
          self.modules.push(module.clone());
        }
      });
  }

  // start build phrase
  pub fn build(&mut self) {
    self.plugin_driver.borrow().build_start(&self.options.borrow());

    self.generate_module_graph();

    self.sort_modules();

    self.include_statements();
  }

  fn include_statements(&self) {
    // TODO: collect statements via entry modules  and tree-shaking.
  }

  fn sort_modules(&mut self) {
    let (cycle_paths, ordered_modules) = analyse_module_execution(&self.entry_modules);

    cycle_paths.iter().for_each(|path| {
      panic!("cyclePaths {:#?}", path);
    });

    self.modules = ordered_modules;

    println!("orderedModules {:#?}", self.modules)
    // (cyclePaths, orderedModules)
  }
}

pub fn normalize_entry_modules(
  entry_modules: Vec<(Option<String>, String)>,
) -> Vec<UnresolvedModule> {
  entry_modules
    .into_iter()
    .map(|(name, id)| {
      UnresolvedModule {
        file_name: None,
        id,
        // implicitlyLoadedAfter: [],
        importer: None,
        name,
      }
    })
    .collect()
}


