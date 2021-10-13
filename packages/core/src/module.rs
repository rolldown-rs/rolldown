use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::sync::atomic::AtomicBool;
use std::sync::RwLock;
use std::sync::{atomic::Ordering, Arc};

use ahash::RandomState;
use log::debug;
use rayon::prelude::*;
use swc_common::{
  errors::{ColorConfig, Handler},
  FileName,
};
use swc_ecma_ast::{ModuleDecl, ModuleItem};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};

use crate::graph::{Graph, ModOrExt};
use crate::statement::Statement;
use crate::{ast, graph};

#[derive(Clone)]
pub struct Module {
  pub source: String,
  pub graph: *const Graph,
  // pub(crate) swc_module: Option<*mut swc_ecma_ast::Module>,
  pub statements: Vec<Arc<RwLock<Statement>>>,
  pub id: String,
  pub imports: HashMap<String, ImportDesc, RandomState>,
  pub exports: HashMap<String, ExportDesc, RandomState>,
  pub defines: HashSet<String>,
}

unsafe impl Sync for Module {}
unsafe impl Send for Module {}

impl Module {
  pub(crate) fn empty() -> Self {
    Self {
      source: "".to_owned(),
      id: "".to_owned(),
      graph: std::ptr::null(),
      statements: vec![],
      imports: HashMap::default(),
      exports: HashMap::default(),
      defines: HashSet::default(),
    }
  }

  pub fn new(source: String, id: String, graph: &Arc<Graph>) -> Self {
    let ast = Module::get_ast(source.clone(), id.clone());
    let statements = ast
      .body
      .into_iter()
      .map(|node| Statement::new(node, id.clone()))
      .map(RwLock::new)
      .map(Arc::new)
      .collect::<Vec<Arc<RwLock<Statement>>>>();

    let defines = statements
      .iter()
      .map(|stmt| {
        stmt
          .read()
          .unwrap()
          .scope
          .defines
          .read()
          .iter()
          .map(|s| s.clone())
          .collect()
      })
      .collect();
    debug!("top defines {:?} id: {:?}", defines, id);
    let module = Module {
      statements,
      source,
      id: id.clone(),
      graph: Arc::as_ptr(&graph),
      imports: HashMap::default(),
      exports: HashMap::default(),
      defines,
    };
    module.analyse();
    module
  }

  fn get_ast(source: String, filename: String) -> swc_ecma_ast::Module {
    let handler = Handler::with_tty_emitter(
      ColorConfig::Auto,
      true,
      false,
      Some(graph::SOURCE_MAP.clone()),
    );
    let fm = graph::SOURCE_MAP.new_source_file(FileName::Custom(filename), source);

    let lexer = Lexer::new(
      // We want to parse ecmascript
      Syntax::Es(Default::default()),
      // JscTarget defaults to es5
      Default::default(),
      StringInput::from(&*fm),
      None,
    );

    let mut parser = Parser::new_from(lexer);

    for e in parser.take_errors() {
      e.into_diagnostic(&handler).emit();
    }

    parser
      .parse_module()
      .map_err(|e| {
        // Unrecoverable fatal error occurred
        e.into_diagnostic(&handler).emit()
      })
      .expect("failed to parser module")
  }

  fn analyse(&self) {
    // analyse imports and exports
    // @TODO
    // Handle duplicated
    // let collects = self.statements
    //   .iter()
    //   .filter(|s| s.is_import_declaration)
    //   .filter_map(|s| {
    //     if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = s.node.as_ref() {
    //       Some(import_decl.specifiers.iter().filter_map(move |specifier| {
    //         let local_name;
    //         let name;
    //         match specifier {
    //           // import foo from './foo'
    //           swc_ecma_ast::ImportSpecifier::Default(n) => {
    //             local_name = n.local.sym.to_string();
    //             name = "default".to_owned();
    //           }
    //           // import { foo } from './foo'
    //           // import { foo as foo2 } from './foo'
    //           swc_ecma_ast::ImportSpecifier::Named(n) => {
    //             local_name = n.local.sym.to_string();
    //             name = n.imported.as_ref().map_or(
    //               local_name.clone(), // `import { foo } from './foo'` doesn't has `imported` name, so we think `local_name` as `imported` name
    //               |ident| ident.sym.to_string(), // `import { foo as _foo } from './foo'` has `imported` name 'foo'
    //             );
    //           }
    //           // import * as foo from './foo'
    //           swc_ecma_ast::ImportSpecifier::Namespace(n) => {
    //             local_name = n.local.sym.to_string();
    //             name = "*".to_owned()
    //           }
    //         }
    //         Some((
    //           local_name.clone(),
    //           ImportDesc {
    //             source: import_decl.src.value.to_string(),
    //             name,
    //             local_name,
    //           },
    //         ))
    //       }))
    //     } else {
    //       None
    //     }
    //   })
    //   .flatten()
    //   .collect();
  }

  pub fn deconflict(&self, statements: &Vec<Statement>) {
    // name => module_id
    let mut definers = HashMap::new();
    // conflict names
    let mut conflicts = HashSet::new();
    statements.iter().for_each(|stmt| {
      stmt.defines.iter().for_each(|name| {
        if definers.contains_key(name) {
          conflicts.insert(name.clone());
        } else {
          definers.insert(name.clone(), stmt.module_id.clone());
        }
      });
    });
  }

  pub fn expand_all_statements(&self, _is_entry_module: bool) -> Vec<Arc<RwLock<Statement>>> {
    // println!("expand_all_modules start from {:?}", self.id);
    // self.is_included.store(true, Ordering::SeqCst);
    // let statements = self
    //   .take_swc_module()
    //   .body
    //   .into_par_iter()
    //   .map(|i| Statement::new(i, self.id.clone()))
    //   .collect::<Vec<Statement>>();

    // self.deconflict(&statements);

    let module_items = self
      .statements
      .iter()
      // FIXME: will cause repated bundle
      // .par_iter()
      .flat_map(|statement| {
        // let read_lock = statement.is_included.read();
        // let is_included = *read_lock.deref();
        // std::mem::drop(read_lock);
        // if is_included {
        //   return vec![];
        // } else {
        //   let mut write_lock = statement.is_included.write();
        //   *write_lock = true;
        //   std::mem::drop(write_lock);
        // }
        if let ModuleItem::ModuleDecl(module_decl) = &statement.read().unwrap().node {
          match module_decl {
            ModuleDecl::Import(import_decl) => {
              if let Ok(ModOrExt::Mod(m)) = Graph::fetch_module(
                &self.get_graph(),
                &import_decl.src.value.to_string(),
                Some(&self.id),
              ) {
                return Module::expand_all_statements(&m, false);
              };
              return vec![];
            }
            ModuleDecl::ExportNamed(node) => {
              // export { foo } from './foo'
              // export { foo as foo2 } from './foo'
              // export * as foo from './foo'
              if let Some(src) = &node.src {
                if let Ok(ModOrExt::Mod(m)) =
                  Graph::fetch_module(&self.get_graph(), &src.value.to_string(), Some(&self.id))
                {
                  return Module::expand_all_statements(&m, false);
                } else {
                  return vec![];
                }
              } else {
                // skip `export { foo, bar, baz }`
                return vec![];
              }
            }
            _ => {}
          }
        }

        ast::helper::fold_export_decl_to_decl(&mut statement.write().unwrap().node);
        Statement::expand(statement)
      })
      .collect();
    debug!("expand_all_modules from {:?}", self.id);
    module_items
  }

  // fn take_swc_module(&self) -> Box<swc_ecma_ast::Module> {
  //   unsafe { Box::from_raw(self.swc_module.unwrap()) }
  // }

  // fn get_swc_module(&self) -> &mut swc_ecma_ast::Module {
  //   unsafe { Box::leak(Box::from_raw(self.swc_module.unwrap())) }
  // }

  fn get_graph(&self) -> Arc<Graph> {
    unsafe {
      Arc::increment_strong_count(self.graph);
      Arc::from_raw(self.graph)
    }
  }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ImportDesc {
  source: String,
  name: String,
  local_name: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ExportDesc {
  name: String,
  local_name: String,
}
