use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use ahash::RandomState;
use log::debug;
use rayon::prelude::*;
use swc_common::sync::RwLock;
use swc_ecma_ast::{ModuleDecl, ModuleItem};

use crate::graph;
use crate::graph::Graph;
use crate::statement::Statement;
use crate::types::{shared, Shared};

use self::analyse::{DynImportDesc, ExportDesc, ImportDesc, ReExportDesc, get_module_info_from_ast, parse_file};
pub mod analyse;

pub struct Module {
  // source code
  pub original_code: Option<String>,
  // statements: Vec<Arc<Statement>>,
  // name => Statement
  // definitions: HashMap<String, Arc<Statement>>,
  // pub modifications: HashMap<String, Vec<Arc<Statement>>>,
  // filename
  pub is_entry: bool,
  pub id: String,
  pub imports: HashMap<String, ImportDesc>,
  pub exports: HashMap<String, ExportDesc>,
  pub dynamic_imports: Vec<DynImportDesc>,
  pub re_exports: HashMap<String, ReExportDesc>,
  pub export_all_sources: HashSet<String>,
  pub sources: HashSet<String>,
  pub resolved_ids: HashMap<String, String>,
  // named re_export. sush as `export { foo } from ...` or `export * as foo from '...'`
  // just re-export. sush as `export * from ...`
  // already included name
  // defined: RwLock<HashSet<String, RandomState>>,
  // suggested name to replace current name
  // suggested_names: RwLock<HashMap<String, String>>,
  pub is_user_defined_entry_point: bool,
}

unsafe impl Sync for Module {}
unsafe impl Send for Module {}

impl Module {
  pub fn new(id: String, is_entry: bool) -> Shared<Self> {
    // let ast = analyse::parse_file(source, id.clone(), &graph::SOURCE_MAP)?;

    // let statements = ast
    //   .body
    //   .into_iter()
    //   .map(|node| Arc::new(Statement::new(node, id.clone())))
    //   .collect::<Vec<Arc<Statement>>>();

    // let (imports, exports, re_exports, export_all_sources) = Module::analyse(&ast.body, &id);

    // let defines = statements
    //   .iter()
    //   .flat_map(|stmt| stmt.defines.clone())
    //   .collect::<Vec<String>>();
    // log::debug!("top defines: {:?}, id: {:?}", defines, id);

    // let definitions = statements
    //   .iter()
    //   .flat_map(|s| s.defines.iter().map(move |d| (d.clone(), s.clone())))
    //   .collect();

    // let mut modifications = HashMap::new();
    // statements.iter().for_each(|s| {
    //   s.modifies.iter().for_each(|name| {
    //     if !modifications.contains_key(name) {
    //       modifications.insert(name.clone(), vec![]);
    //     }
    //     if let Some(vec) = modifications.get_mut(name) {
    //       vec.push(s.clone());
    //     }
    //   })
    // });

    shared(Module {
      // statements,
      original_code: None,
      id,
      is_entry,
      imports: HashMap::default(),
      exports: HashMap::default(),
      re_exports: HashMap::default(),
      dynamic_imports: Vec::default(),
      export_all_sources: HashSet::default(),
      sources: HashSet::default(),
      resolved_ids: HashMap::default(),
      is_user_defined_entry_point: false,
      // definitions,
      // modifications,
      // defined: RwLock::new(HashSet::default()),
      // suggested_names: RwLock::new(HashMap::default()),
    })
  }

  // #[inline]
  // pub fn expand_all_statements(&self, is_entry_module: bool, graph: &Graph) -> Vec<Arc<Statement>> {
  //   log::debug!("expand_all_statements {:?}", self.id);

  //   let all_statements = self
  //     .statements
  //     .iter()
  //     .flat_map(|s| {
  //       if s.is_import_declaration {
  //         vec![]
  //       } else {
  //         let read_lock = s.node.read();
  //         let statement: &ModuleItem = &read_lock;
  //         if let ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(node)) = statement {
  //           if !node.specifiers.is_empty() && is_entry_module {
  //             s.expand(self, graph)
  //           } else {
  //             vec![]
  //           }
  //         } else {
  //           s.expand(self, graph)
  //         }
  //       }
  //     })
  //     .collect::<Vec<Arc<Statement>>>();

  //   all_statements
  // }

  // pub fn define(&self, local_name: &str, graph: &Graph) -> Vec<Arc<Statement>> {
  //   if self.defined.read().contains(local_name) {
  //     log::debug!("already define {} in module {}", local_name, self.id);
  //     vec![]
  //   } else {
  //     let result;
  //     // check the name whether is imported from other module
  //     if let Some(import_desc) = self.imports.get(local_name) {
  //       let module = graph
  //         .fetch_module(&import_desc.source, Some(&self.id))
  //         .expect("Exist")
  //         .into_mod()
  //         .expect("A module");
  //       log::debug!(
  //         "define {}, which is {} from {}",
  //         local_name,
  //         import_desc.name,
  //         module.id
  //       );
  //       // case: import { default as foo } from './foo'
  //       if import_desc.name == "default" {
  //         let local_name = import_desc.local_name.clone();
  //         let mut suggestion = self
  //           .suggested_names
  //           .read()
  //           .get(&local_name)
  //           .map_or(local_name.clone(), |n| n.clone());
  //         while module.imports.contains_key(&suggestion) {
  //           suggestion.push('_');
  //         }
  //         module.suggest_name("default".to_owned(), suggestion);
  //         // case: import { * as foo } from './foo'
  //       } else if import_desc.name == "*" {
  //         let local_name = import_desc.local_name.clone();
  //         let suggestion = self
  //           .suggested_names
  //           .read()
  //           .get(&local_name)
  //           .map_or(local_name.clone(), |n| n.clone());
  //         module.suggest_name("*".to_owned(), suggestion.clone());
  //         module.suggest_name("default".to_owned(), suggestion + "__default");

  //         graph.insert_internal_namespace_module_id(module.id.clone());
  //         result = module.expand_all_statements(false, graph);
  //         return result;
  //       }

  //       // check the name whether is exported from other module
  //       if let Some(export_desc) = module.exports.get(&import_desc.name) {
  //         let name = match export_desc {
  //           ExportDesc::Decl(node) => &node.local_name,
  //           ExportDesc::Default(node) => &node.local_name,
  //           ExportDesc::Named(node) => &node.local_name,
  //         };
  //         result = module.define(name, graph);
  //       } else {
  //         panic!(
  //           "Module {:?} does not export {:?} (imported by {:?})",
  //           module.id, import_desc.name, self.id
  //         )
  //       }
  //     } else if local_name == "default"
  //       && self
  //         .exports
  //         .get("default")
  //         .map(|v| v.has_identifier())
  //         .unwrap_or(false)
  //     {
  //       let name = match self.exports.get("default").as_ref().unwrap() {
  //         ExportDesc::Default(node) => node.identifier.as_ref().unwrap(),
  //         _ => panic!("bug"),
  //       };
  //       result = self.define(name, graph)
  //       // result = vec![];
  //     } else {
  //       let statement;
  //       if local_name == "default" {
  //         if let Some(ExportDesc::Default(node)) = self.exports.get("default") {
  //           statement = Some(&node.statement)
  //         } else {
  //           statement = self.definitions.get(local_name)
  //         }
  //       } else {
  //         statement = self.definitions.get(local_name)
  //       }

  //       if statement.is_none() {
  //         debug!("detect global name {} in {}", local_name, self.id)
  //       }

  //       result = statement.map_or(vec![], |s| s.expand(self, graph));
  //     }
  //     self.defined.write().insert(local_name.to_string());

  //     log::debug!(
  //       "define {} with {} statements in module {}",
  //       local_name,
  //       result.len(),
  //       self.id
  //     );

  //     result
  //   }
  // }

  // pub fn get_canonical_name(&self, raw_local_name: &str) -> String {
  //   raw_local_name.to_owned()
  // }

  // pub fn suggest_name(&self, name: String, suggestion: String) {
  //   self.suggested_names.write().insert(name, suggestion);
  // }
}

impl Module {
  pub fn set_source(&mut self, source: String) {
    let ast = parse_file(source, self.id.clone(), &graph::SOURCE_MAP).unwrap();
    let module_info = get_module_info_from_ast(&ast, self.id.clone());

    self.imports = module_info.imports;
    self.exports = module_info.exports;
    self.export_all_sources = module_info.export_all_sources;
    self.dynamic_imports = module_info.dyn_imports;
    self.sources = module_info.sources;
  }

  pub fn update_options(&self) {}
}
