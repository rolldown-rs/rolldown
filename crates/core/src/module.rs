use std::fmt::Debug;

use hashbrown::{HashMap, HashSet};
use linked_hash_set::LinkedHashSet;
use swc_atoms::JsWord;
use swc_common::Mark;
use swc_ecma_ast::Id;

use crate::ModuleGraph;

pub struct JsModule {
    pub id: JsWord,
    pub ast: swc_ecma_ast::Program,
    pub dependecies: LinkedHashSet<JsWord>,
    pub dyn_dependecies: HashSet<JsWord>,
    pub top_level_mark: Mark,
    pub imports: HashMap<JsWord, HashSet<Id>>,
    pub re_exports: HashMap<JsWord, HashSet<Id>>,
    pub re_export_all: HashMap<JsWord, HashSet<Id>>,
    pub local_exports: HashSet<Id>,
    pub final_exports: HashMap<JsWord, HashSet<Id>>,
    
    // pub dependencies: LinkedHashSet<JsWord>,
    // pub imports: HashMap<JsWord, HashSet<Id>>
    // pub exports
}

impl JsModule {
    pub fn depended_modules<'a>(&self, module_graph: &'a ModuleGraph) -> Vec<&'a JsModule> {
        module_graph.dependecies_by_module(self)
    }

    pub fn dynamic_depended_modules<'a>(&self, module_graph: &'a ModuleGraph) -> Vec<&'a JsModule> {
        module_graph.dyn_dependecies_by_module(self)
    }
}

impl Debug for JsModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsModule")
            .field("id", &self.id)
            .field("ast", &"...")
            .field("dependecies", &self.dependecies)
            .field("dyn_dependecies", &self.dyn_dependecies)
            .finish()
    }
}
