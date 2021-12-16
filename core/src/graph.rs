use std::collections::{HashMap, HashSet};

use crate::types::{shared, NormalizedInputOptions, Shared};
use crate::utils::path::relative_id;
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


fn analyse_module(
  module: &ModOrExt,
  nextExecIndex: &mut i32,
  cyclePaths: &mut Vec<Vec<String>>,
  analysedModules:&mut HashSet<ModOrExt>,
  dynamicImports:&mut HashSet<Shared<Module>>,
  parents:&mut HashMap<ModOrExt, Option<Shared<Module>>>,
  orderedModules:&mut Vec<Shared<Module>>,
) {
  if let ModOrExt::Mod(module) = module {
    module.borrow().dependencies.iter().for_each(|dependency| {
      if parents.contains_key(dependency) {
        if !analysedModules.contains(dependency) {
          cyclePaths.push(get_cycle_path(&dependency.clone().into_mod().unwrap(), module, parents));
        }
        return;
      }
      parents.insert(dependency.clone(), Some(module.clone()));
      analyse_module(
        &dependency,
          nextExecIndex,
          cyclePaths,
          analysedModules,
          dynamicImports,
          parents,
          orderedModules,
      );
    });

    // for (const dependency of module.implicitlyLoadedBefore) {
    //   dynamicImports.add(dependency);
    // }

    module.borrow().dynamic_imports.iter().for_each(|dyn_import| {
      if let Some(ModOrExt::Mod(resolution)) = &dyn_import.resolution {
        dynamicImports.insert(resolution.clone());
      }
    });
    orderedModules.push(module.clone());
  }

  // module.execIndex = nextExecIndex++;
  *nextExecIndex += 1; 
  analysedModules.insert(module.clone());
}

fn get_cycle_path(
	module: &Shared<Module>,
	parent: &Shared<Module>,
	parents: &HashMap<ModOrExt, Option<Shared<Module>>>
) -> Vec<String> {
	// const cycle_symbol = Symbol(module.id);
  	let cycle_symbol = &module.borrow().id;

	let mut path = vec![relative_id(cycle_symbol.clone())];
	let mut maybeNextModule = Some(parent.clone());
	// module.cycles.add(cycleSymbol);
	while let Some(next_odule) = &maybeNextModule {
    if next_odule != module {
      // nextModule.cycles.add(cycleSymbol);
      path.push(relative_id(next_odule.borrow().id.clone()));
      maybeNextModule = parents.get(&next_odule.clone().into()).unwrap().clone()
    } else {
      break;
    }
		
	}
	// while (nextModule !== module) {
	// 	nextModule.cycles.add(cycleSymbol);
	// 	path.push(relativeId(nextModule.id));
	// 	nextModule = parents.get(nextModule)!;
	// }
	path.push(relative_id(cycle_symbol.clone()));
	path.reverse();
	return path;
}


fn analyse_module_execution(entry_modules: &[Shared<Module>]) -> (Vec<Vec<String>>, Vec<Shared<Module>>) {
  // TODO: sort modules and analyze cycle imports
  let mut nextExecIndex = 0;
  let mut cyclePaths: Vec<Vec<String>> = vec![];
  let mut analysedModules: HashSet<ModOrExt> = HashSet::new();
  let mut dynamicImports: HashSet<Shared<Module>> = HashSet::new();
  let mut parents: HashMap<ModOrExt, Option<Shared<Module>>> = HashMap::default();
  let mut orderedModules: Vec<Shared<Module>> = vec![];

  entry_modules.iter().for_each(|cur_entry| {
    if !parents.contains_key(&cur_entry.clone().into()) {
      parents.insert(cur_entry.clone().into(), None);
      analyse_module(
        &cur_entry.clone().into(),
      &mut nextExecIndex,
      &mut cyclePaths,
      &mut analysedModules,
      &mut dynamicImports,
      &mut parents,
      &mut orderedModules,
      );
    }
  });

  let unsafe_dynamicImports = unsafe {
    let p = &mut dynamicImports as *mut HashSet<Shared<Module>>;
    p.as_mut().unwrap()
  };

  dynamicImports.iter().for_each(|curEntry| {
    if !parents.contains_key(&curEntry.clone().into()) {
      parents.insert(curEntry.clone().into(), None);
      analyse_module(
        &curEntry.clone().into(),
      &mut nextExecIndex,
      &mut cyclePaths,
      &mut analysedModules,
      unsafe_dynamicImports,
      &mut parents,
      &mut orderedModules,
      );
    }

  });


  (cyclePaths, orderedModules)
}