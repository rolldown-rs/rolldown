use std::collections::{HashSet, HashMap};

use crate::{types::{ModOrExt, Shared}, Module};

use super::path::relative_id;

fn analyse_module(
  module: &ModOrExt,
  next_exec_index: &mut usize,
  cycle_paths: &mut Vec<Vec<String>>,
  analysed_modules:&mut HashSet<ModOrExt>,
  dynamic_imports:&mut HashSet<Shared<Module>>,
  parents:&mut HashMap<ModOrExt, Option<Shared<Module>>>,
  ordered_modules:&mut Vec<Shared<Module>>,
) {
  if let ModOrExt::Mod(module) = module {
    module.borrow().dependencies.iter().for_each(|dependency| {
      if parents.contains_key(dependency) {
        if !analysed_modules.contains(dependency) {
          cycle_paths.push(get_cycle_path(&dependency.clone().into_mod().unwrap(), module, parents));
        }
        return;
      }
      parents.insert(dependency.clone(), Some(module.clone()));
      analyse_module(
        &dependency,
          next_exec_index,
          cycle_paths,
          analysed_modules,
          dynamic_imports,
          parents,
          ordered_modules,
      );
    });

    // for (const dependency of module.implicitlyLoadedBefore) {
    //   dynamicImports.add(dependency);
    // }

    module.borrow().dynamic_imports.iter().for_each(|dyn_import| {
      if let Some(ModOrExt::Mod(resolution)) = &dyn_import.resolution {
        dynamic_imports.insert(resolution.clone());
      }
    });
    ordered_modules.push(module.clone());
  }

  *next_exec_index += 1; 
  module.set_exec_index(*next_exec_index);
  // module.execIndex = nextExecIndex++;
  analysed_modules.insert(module.clone());
}

fn get_cycle_path(
	module: &Shared<Module>,
	parent: &Shared<Module>,
	parents: &HashMap<ModOrExt, Option<Shared<Module>>>
) -> Vec<String> {
	// const cycle_symbol = Symbol(module.id);
  	let cycle_symbol = &module.borrow().id;

	let mut path = vec![relative_id(cycle_symbol.clone())];
	let mut maybe_next_module = Some(parent.clone());
	module.borrow_mut().cycles.insert(cycle_symbol.clone());
	while let Some(next_odule) = &maybe_next_module {
    if next_odule != module {
      next_odule.borrow_mut().cycles.insert(cycle_symbol.clone());
      path.push(relative_id(next_odule.borrow().id.clone()));
      maybe_next_module = parents.get(&next_odule.clone().into()).unwrap().clone()
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
  let mut next_exec_index = 0;
  let mut cycle_paths: Vec<Vec<String>> = vec![];
  let mut analysed_modules: HashSet<ModOrExt> = HashSet::new();
  let mut dynamic_imports: HashSet<Shared<Module>> = HashSet::new();
  let mut parents: HashMap<ModOrExt, Option<Shared<Module>>> = HashMap::default();
  let mut ordered_modules: Vec<Shared<Module>> = vec![];

  entry_modules.iter().for_each(|cur_entry| {
    if !parents.contains_key(&cur_entry.clone().into()) {
      parents.insert(cur_entry.clone().into(), None);
      analyse_module(
        &cur_entry.clone().into(),
      &mut next_exec_index,
      &mut cycle_paths,
      &mut analysed_modules,
      &mut dynamic_imports,
      &mut parents,
      &mut ordered_modules,
      );
    }
  });

  let unsafe_dynamic_imports = unsafe {
    let p = &mut dynamic_imports as *mut HashSet<Shared<Module>>;
    p.as_mut().unwrap()
  };

  dynamic_imports.iter().for_each(|cur_entry| {
    if !parents.contains_key(&cur_entry.clone().into()) {
      parents.insert(cur_entry.clone().into(), None);
      analyse_module(
        &cur_entry.clone().into(),
      &mut next_exec_index,
      &mut cycle_paths,
      &mut analysed_modules,
      unsafe_dynamic_imports,
      &mut parents,
      &mut ordered_modules,
      );
    }

  });


  (cycle_paths, ordered_modules)
}