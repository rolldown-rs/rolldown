use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{atomic::Ordering, Arc};

use ahash::RandomState;
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
  pub(crate) swc_module: Option<swc_ecma_ast::Module>,
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
      swc_module: Some(Module::get_ast(source.clone(), id.clone())),
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
        let module_item = &s.node;
        if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = module_item {
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

  pub fn expand_all_modules(this: Arc<Self>, _is_entry_module: bool) -> Vec<Arc<Self>> {
    this
      .swc_module
      .as_ref()
      .unwrap()
      .body
      .iter()
      .filter_map(|module_item| {
        if let ModuleItem::ModuleDecl(module_decl) = module_item {
          match module_decl {
            ModuleDecl::Import(import_decl) => {
              if let Ok(ModOrExt::Mod(m)) = Graph::fetch_module(
                &this.get_graph(),
                &import_decl.src.value.to_string(),
                Some(&this.id),
              ) {
                if m.is_included.load(Ordering::Relaxed) {
                  return None;
                }
                return Some(Module::expand_all_modules(m, false));
              };
              return None;
            }
            ModuleDecl::ExportNamed(node) => {
              // export { foo } from './foo'
              // export { foo as foo2 } from './foo'
              // export * as foo from './foo'
              if let Some(src) = &node.src {
                if let Ok(ModOrExt::Mod(m)) =
                  Graph::fetch_module(&this.get_graph(), &src.value.to_string(), Some(&this.id))
                {
                  return Some(Module::expand_all_modules(m, false));
                };
              }
              return None;
            }
            _ => {}
          }
        }

        this.expand();
        Some(vec![this.clone()])
      })
      .flatten()
      .collect()
  }

  fn expand(&self) {
    self.is_included.store(true, Ordering::Relaxed);
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
