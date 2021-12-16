use std::collections::{HashSet, HashMap};

use crate::{types::{ModOrExt, Shared}, Module};

use super::path::relative_id;

fn analyse_module(
  module: &ModOrExt,
  nextExecIndex: &mut usize,
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

  *nextExecIndex += 1; 
  module.set_exec_index(*nextExecIndex);
  // module.execIndex = nextExecIndex++;
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
	module.borrow_mut().cycles.insert(cycle_symbol.clone());
	while let Some(next_odule) = &maybeNextModule {
    if next_odule != module {
      next_odule.borrow_mut().cycles.insert(cycle_symbol.clone());
      path.push(relative_id(next_odule.borrow().id.clone()));
      maybeNextModule = parents.get(&next_odule.clone().into()).unwrap().clone()
    } else {
      break;
    }
		
	}
	path.push(relative_id(cycle_symbol.clone()));
	path.reverse();
	return path;
}


pub fn analyse_module_execution(entry_modules: &[Shared<Module>]) -> (Vec<Vec<String>>, Vec<Shared<Module>>) {
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