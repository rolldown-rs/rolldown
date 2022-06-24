use std::collections::{HashMap, HashSet};

use swc_atoms::JsWord;

use crate::{module::JsModule, Dependency};

#[derive(Debug, Default)]
pub struct ModuleGraph {
    pub(crate) id_to_module: HashMap<String, JsModule>,
    pub(crate) resolved_id_map: HashMap<Option<JsWord>, HashMap<JsWord, JsWord>>,
    // id_to_uri: hashbrown::HashMap<String, String>,
}

impl ModuleGraph {
    pub fn add_module(&mut self, module: JsModule) {
        self.id_to_module
            .insert(module.id.clone().to_string(), module);
    }

    pub fn add_dependency(&mut self, importer: Option<JsWord>, importee: (JsWord, JsWord)) {
        self.resolved_id_map
            .entry(importer)
            .or_default()
            .insert(importee.0, importee.1);
    }

    pub fn module_by_id(&self, id: &str) -> Option<&JsModule> {
        self.id_to_module.get(id)
    }

    pub fn module_by_id_mut(&mut self, id: &str) -> Option<&mut JsModule> {
        self.id_to_module.get_mut(id)
    }

    pub fn dependecies_by_module(&self, js_mod: &JsModule) -> Vec<&JsModule> {
        js_mod
            .dependecies
            .iter()
            .filter_map(|relative_src| self.resolved_id_map[&Some(js_mod.id.clone())].get(relative_src))
            .filter_map(|id| self.module_by_id(id))
            .collect()
    }

    pub fn dyn_dependecies_by_module(&self, js_mod: &JsModule) -> Vec<&JsModule> {
      js_mod
            .dyn_dependecies
            .iter()
            .filter_map(|relative_src| self.resolved_id_map[&Some(js_mod.id.clone())].get(relative_src))
            .filter_map(|id| self.module_by_id(id))
            .collect()
    }

    // pub fn module_by_dependency_mut(&mut self, dep: &Dependency) -> Option<&mut ModuleGraphModule> {
    //   let uri = self.dependency_to_module_uri.get(dep)?;
    //   self.uri_to_module.get_mut(uri)
    // }

    pub fn modules(&self) -> impl Iterator<Item = &JsModule> {
        self.id_to_module.values()
    }

    // pub fn deps_by_id(&self, id: &str) -> Vec<>

    // pub fn remove_by_id(&mut self, id: &str) -> Option<Module> {
    //   let uri = self.id_to_uri.get(id)?;
    //   let js_mod = self.uri_to_module.remove(uri)?;
    //   self.id_to_uri.remove(id);
    //   Some(js_mod)
    // }
}
