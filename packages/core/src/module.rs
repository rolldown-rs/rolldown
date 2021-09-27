use std::collections::HashMap;
use std::sync::{atomic::Ordering, Arc};

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
      id: id,
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
    let mut imports = HashMap::new();
    // analyse imports and exports
    self
      .statements
      .iter()
      .filter(|s| s.is_import_declaration)
      .map(|s| &s.node)
      .for_each(|module_item| {
        if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = module_item {
          import_decl.specifiers.iter().for_each(|specifier| {
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
            if imports.contains_key(&local_name) {
              panic!("Duplicated import {:?}", local_name);
            }
            imports.insert(
              local_name.clone(),
              ImportDesc {
                source: import_decl.src.value.to_string(),
                name,
                local_name,
              },
            );
          });
        }
      });

    self.imports = imports;
  }

  pub fn expand_all_statements(&self, _is_entry_module: bool) -> Vec<Arc<Statement>> {
    let mut all_statements: Vec<Arc<Statement>> = vec![];
    self.statements.iter().for_each(|statement| {
      if statement.is_included.load(Ordering::Relaxed) {
        return;
      }

      if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = &statement.node {
        // import './effects'
        if import_decl.specifiers.is_empty() {
        } else {
          if let Ok(ModOrExt::Mod(ref m)) = Graph::fetch_module(
            &self.graph,
            &import_decl.src.value.to_string(),
            Some(&self.id),
          ) {
            let mut statements = m.expand_all_statements(false);
            all_statements.append(&mut statements);
          };
        }
        return;
      }
      statement.expand();
      all_statements.push(statement.clone());

      // TODO: // skip `export { foo, bar, baz }`...
    });
    all_statements
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
