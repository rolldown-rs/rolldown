// use std::collections::HashMap;
//
// use swc_atoms::JsWord;
// use swc_ecma_ast::{Ident, Module as SwcModule};
// use swc_ecma_visit::{VisitMut, VisitMutWith};
//
// use crate::module::Module;
//
// use crate::graph::{DepGraph, DepNode};
//
// pub struct Linker<'a> {
//   pub module: &'a Module,
//   pub graph: &'a DepGraph,
//   // pub asserted_globals: &'a HashMap<JsWord, bool>,
// }
//
// impl<'a> Linker<'a> {
//   fn get_importee_module(&self, source: JsWord) -> &DepNode {
//     let idx = self
//       .graph
//       .node_indices()
//       .position(|idx| match &self.graph[idx] {
//         DepNode::Mod(m) => m.id.as_str() == source.as_ref(),
//         DepNode::Ext(m) => return false,
//       })
//       .unwrap();
//
//     self.graph[idx]
//   }
// }
//
// impl<'a> VisitMut for Linker<'a> {
//   fn visit_mut_ident(&mut self, i: &mut Ident) {
//     println!("ident {:#?}", i.to_string());
//     // asserted as global or imported from other modules
//     if let Some(ref scan) = self.module.scanner {
//       if let Some(import_desc) = scan.imports.get(&i.sym) {
//         // TODO: iterate through all related bindings
//       }
//     }
//     // else if self.module.definitions.get(&i.sym).is_none() {
//     //   self.asserted_globals.insert(&i.sym).is_none();
//     // }
//   }
// }
