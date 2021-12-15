use crate::{
  types::{ModOrExt, Shared},
  Module,
};

use std::collections::{HashMap, HashSet};

pub struct ChunkDefinition {
  pub alias: Option<String>,
  pub modules: Vec<Shared<Module>>,
}

type ChunkDefinitions = Vec<ChunkDefinition>;

type DependentModuleMap = HashMap<Shared<Module>, HashSet<Shared<Module>>>;

pub fn get_chunk_assignments(
  _entry_modules: &[Shared<Module>],
  manual_chunk_alias_by_entry: &HashMap<Shared<Module>, String>,
) -> ChunkDefinitions {
  let mut chunk_definitions = vec![];

  let mut modules_in_manual_chunks = manual_chunk_alias_by_entry
    .keys()
    .map(|m| m.clone())
    .collect::<HashSet<Shared<Module>>>();

  let mut manual_chunk_modules_by_alias: HashMap<String, Vec<Shared<Module>>> = HashMap::default();

  manual_chunk_alias_by_entry
    .iter()
    .for_each(|(entry, alias)| {
      let chunks_modules = manual_chunk_modules_by_alias
        .entry(alias.clone())
        .or_insert(Vec::new());

      add_static_dependencies_to_manual_chunk(
        entry.clone(),
        chunks_modules,
        &mut modules_in_manual_chunks,
      );
    });

  manual_chunk_modules_by_alias
    .into_iter()
    .for_each(|(alias, modules)| {
      chunk_definitions.push(ChunkDefinition {
        alias: Some(alias),
        modules,
      });
    });

  let _assigned_entry_points_by_module: DependentModuleMap = HashMap::default();

  chunk_definitions
}

fn analyze_module_graph(
  entry_modules: &[Shared<Module>],
) -> (DependentModuleMap, HashSet<Shared<Module>>) {
  // let dynamic_entry_modules = HashSet::new();
  let _dependent_entry_points_by_module: DependentModuleMap = HashMap::default();
  let _entries_to_handle = entry_modules
    .iter()
    .map(|m| m.clone())
    .collect::<HashSet<Shared<Module>>>();
  // entries_to_handle.iter().for_each(|current_entry| {
  //   let mut modules_to_handle = HashSet::new();
  //   let mut rest_modules_to_handle = vec![current_entry.clone()];
  //   while rest_modules_to_handle.len() > 0 {
  //     let module = rest_modules_to_handle.pop().unwrap();
  //     dependent_entry_points_by_module
  //       .entry(module.clone())
  //       .or_insert(HashSet::new())
  //       .insert(current_entry.clone());
  //     module
  //       .borrow()
  //       .get_dependencies_to_be_included()
  //       .into_iter()
  //       .for_each(|dep| {
  //         if let ModOrExt::Mod(dep) = dep {
  //           if !modules_to_handle.contains(&dep) {
  //             modules_to_handle.insert(dep.clone());
  //             rest_modules_to_handle.push(dep);
  //           }
  //         }
  //       })
  //   }
  //   modules_to_handle.into_iter().for_each(|module| {})
  // });
  todo!()
}

fn add_static_dependencies_to_manual_chunk(
  entry: Shared<Module>,
  manual_chunk_modules: &mut Vec<Shared<Module>>,
  modules_in_manual_chunks: &mut HashSet<Shared<Module>>,
) {
  let mut modules_to_handle = HashSet::new();
  modules_to_handle.insert(entry);
  modules_to_handle.iter().for_each(|module| {
    modules_in_manual_chunks.insert(module.clone());
    manual_chunk_modules.push(module.clone());
    module.borrow().dependencies.iter().for_each(|dep| {
      if let ModOrExt::Mod(m) = dep {
        if !modules_in_manual_chunks.contains(m) {
          modules_in_manual_chunks.insert(m.clone());
        }
      }
    });
  });
}
