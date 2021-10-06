use std::collections::HashMap;
use std::ops::Deref;
use std::sync::atomic::AtomicBool;
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

use crate::graph;
use crate::graph::{Graph, ModOrExt};
use crate::statement::Statement;

#[derive(Clone)]
pub struct Module {
  pub source: String,
  pub graph: *const Graph,
  pub(crate) swc_module: Option<*mut swc_ecma_ast::Module>,
  pub id: String,
  pub imports: HashMap<String, ImportDesc, RandomState>,
  pub exports: HashMap<String, ExportDesc, RandomState>,
  is_included: Arc<AtomicBool>,
}

unsafe impl Sync for Module {}
unsafe impl Send for Module {}

impl Module {
  pub(crate) fn empty() -> Self {
    Self {
      source: "".to_owned(),
      id: "".to_owned(),
      graph: std::ptr::null(),
      swc_module: None,
      imports: HashMap::default(),
      exports: HashMap::default(),
      is_included: Arc::new(AtomicBool::new(false)),
    }
  }

  pub fn new(source: String, id: String, graph: &Arc<Graph>) -> Self {
    let module = Module {
      swc_module: Some(Box::into_raw(Box::new(Module::get_ast(
        source.clone(),
        id.clone(),
      )))),
      source,
      id: id.clone(),
      graph: Arc::as_ptr(&graph),
      is_included: Arc::new(AtomicBool::new(false)),
      imports: HashMap::default(),
      exports: HashMap::default(),
    };
    // module.imports = Module::analyse(&module.statements);
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

  fn analyse(statements: &[Arc<Statement>]) -> HashMap<String, ImportDesc, RandomState> {
    // analyse imports and exports
    // @TODO
    // Handle duplicated
    statements
      .iter()
      .filter(|s| s.is_import_declaration)
      .filter_map(|s| {
        if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = s.get_node() {
          Some(import_decl.specifiers.iter().filter_map(move |specifier| {
            let local_name;
            let name;
            match specifier {
              // import foo from './foo'
              swc_ecma_ast::ImportSpecifier::Default(n) => {
                local_name = n.local.sym.to_string();
                name = "default".to_owned();
              }
              // import { foo } from './foo'
              // import { foo as foo2 } from './foo'
              swc_ecma_ast::ImportSpecifier::Named(n) => {
                local_name = n.local.sym.to_string();
                name = n.imported.as_ref().map_or(
                  local_name.clone(), // `import { foo } from './foo'` doesn't has local name
                  |ident| ident.sym.to_string(), // `import { foo as _foo } from './foo'` has local name '_foo'
                );
              }
              // import * as foo from './foo'
              swc_ecma_ast::ImportSpecifier::Namespace(n) => {
                local_name = n.local.sym.to_string();
                name = "*".to_owned()
              }
            }
            Some((
              local_name.clone(),
              ImportDesc {
                source: import_decl.src.value.to_string(),
                name,
                local_name,
              },
            ))
          }))
        } else {
          None
        }
      })
      .flatten()
      .collect()
  }

  pub fn expand_all_modules(&self, _is_entry_module: bool) -> Vec<swc_ecma_ast::ModuleItem> {
    // println!("expand_all_modules start from {:?}", self.id);
    self.is_included.store(true, Ordering::SeqCst);
    
    let module_items = self
    .take_swc_module()
    .body
    .into_par_iter()
    .map(|i| Statement::new(i))
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
        if let ModuleItem::ModuleDecl(module_decl) = statement.get_node() {
          match module_decl {
            ModuleDecl::Import(import_decl) => {
              if let Ok(ModOrExt::Mod(m)) = Graph::fetch_module(
                &self.get_graph(),
                &import_decl.src.value.to_string(),
                Some(&self.id),
              ) {
                if m.is_included.load(Ordering::SeqCst) {
                  return vec![];
                }
                return Module::expand_all_modules(&m, false);
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
                  if m.is_included.load(Ordering::SeqCst) {
                    return vec![];
                  }
                  return Module::expand_all_modules(&m, false);
                };
              }
              return vec![];
            }
            _ => {}
          }
        }
        vec![statement.take_node()]
      })
      .collect();
      debug!("expand_all_modules from {:?}, is_included {:?}", self.id, self.is_included);
      module_items
  }

  fn take_swc_module(&self) -> Box<swc_ecma_ast::Module> {
    unsafe { Box::from_raw(self.swc_module.unwrap()) }
  }

  fn get_swc_module(&self) -> &mut swc_ecma_ast::Module {
    unsafe { Box::leak(Box::from_raw(self.swc_module.unwrap())) }
  }

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
