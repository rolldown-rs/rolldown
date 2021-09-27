use std::collections::HashMap;
use std::sync::{atomic::Ordering, Arc};

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
  pub graph: Arc<Graph>,
  pub id: String,
  pub statements: Vec<Arc<Statement>>,
  pub imports: HashMap<String, ImportDesc>,
  pub exports: HashMap<String, ExportDesc>,
}

impl Module {
  pub fn new(source: String, id: String, graph: Arc<Graph>) -> Self {
    let mut module = Module {
      graph,
      source,
      id,
      statements: vec![],
      imports: HashMap::new(),
      exports: HashMap::new(),
    };

    let ast = module.get_ast();
    let statements = ast
      .body
      .into_iter()
      .map(|node| Arc::new(Statement::new(node)))
      .collect::<Vec<_>>();
    module.statements = statements;

    module.analyse();
    module
  }

  pub fn analyse(&mut self) {
    // analyse imports and exports
    // @TODO
    // Handle duplicated
    self.imports = self
      .statements
      .par_iter()
      .filter(|s| s.is_import_declaration)
      .filter_map(|s| {
        let module_item = &s.node;
        if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = module_item {
          Some(
            import_decl
              .specifiers
              .par_iter()
              .filter_map(move |specifier| {
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
              }),
          )
        } else {
          None
        }
      })
      .flatten()
      .collect()
  }

  pub fn expand_all_statements(&self, _is_entry_module: bool) -> Vec<Arc<Statement>> {
    self
      .statements
      .par_iter()
      .filter_map(|statement| {
        if statement.is_included.load(Ordering::Relaxed) {
          return None;
        }
        if let ModuleItem::ModuleDecl(module_decl) = &statement.node {
          match module_decl {
            ModuleDecl::Import(import_decl) => {
              // TODO: delete unused `import './foo'` that has no effects
              if let Ok(ModOrExt::Mod(ref m)) = Graph::fetch_module(
                &self.graph,
                &import_decl.src.value.to_string(),
                Some(&self.id),
              ) {
                return Some(m.expand_all_statements(false));
              };
              return None;
            }
            ModuleDecl::ExportNamed(node) => {
              // export { foo } from './foo'
              // export { foo as foo2 } from './foo'
              // export * as foo from './foo'
              if let Some(src) = &node.src {
                if let Ok(ModOrExt::Mod(ref m)) =
                  Graph::fetch_module(&self.graph, &src.value.to_string(), Some(&self.id))
                {
                  return Some(m.expand_all_statements(false));
                };
              }
              return None;
            }
            _ => {}
          }
        }

        statement.expand();
        Some(vec![statement.clone()])
      })
      .flatten()
      .collect()
  }

  pub fn get_ast(&self) -> swc_ecma_ast::Module {
    let handler = Handler::with_tty_emitter(
      ColorConfig::Auto,
      true,
      false,
      Some(graph::SOURCE_MAP.clone()),
    );
    let fm =
      graph::SOURCE_MAP.new_source_file(FileName::Custom(self.id.clone()), self.source.clone());

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
