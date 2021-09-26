use std::collections::HashMap;

use swc_common::{
  errors::{ColorConfig, Handler},
  FileName,
};
use swc_ecma_ast::{ModuleDecl, ModuleItem};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};

use crate::graph;
use crate::graph::{Graph, ModOrExt};
use crate::statement::Statement;
use crate::types::shared::Shared;

#[derive(Clone)]
pub struct Module {
  pub source: String,
  pub graph: Shared<Graph>,
  pub id: String,
  pub statements: Vec<Shared<Statement>>,
  pub imports: HashMap<String, ImportDesc>,
  pub exports: HashMap<String, ExportDesc>,
}

impl Module {
  pub fn new(source: String, id: String, graph: Shared<Graph>) -> Self {
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
      .map(|node| {
        let statement = Statement::new(node);
        Shared::new(statement)
      })
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

  pub fn expand_all_statements(&self, _is_entry_module: bool) -> Vec<Shared<Statement>> {
    let mut all_statements: Vec<Shared<Statement>> = vec![];
    self.statements.iter().for_each(|statement| {
      if statement.is_included {
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
            let mut statements = m.borrow_mut().expand_all_statements(false);
            all_statements.append(&mut statements);
          };
        }
        return;
      }

      all_statements.push(Statement::expand(statement));

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
  // module: Option<Shared<Module>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ExportDesc {
  name: String,
  local_name: String,
}

mod tests {

  #[test]
  fn imports_test() {
    // let fake_bundle = Bundle::new("./fake");
    // let code = r#"
    //   import foo from './foo'
    //   import { bar, bar2 as _bar2 } from './bar'
    //   import * as baz from './baz'
    // "#
    // .to_owned();
    // let fake_path = "./fake".to_owned();
    // let m = Module::new(ModuleOptions {
    //   source: code.clone(),
    //   id: fake_path.to_owned(),
    //   bundle: Shared::new(fake_bundle),
    // });

    // let left = m.imports.get("foo").unwrap();
    // let right = &ImportDesc {
    //   source: "./foo".into(),
    //   name: "default".into(),
    //   local_name: "foo".into(),
    // };
    // assert_eq!(left, right);
    // let left = m.imports.get("bar").unwrap();
    // let right = &ImportDesc {
    //   source: "./bar".into(),
    //   name: "bar".into(),
    //   local_name: "bar".into(),
    // };
    // assert_eq!(left, right);
    // let left = m.imports.get("_bar2").unwrap();
    // let right = &ImportDesc {
    //   source: "./bar".into(),
    //   name: "bar2".into(),
    //   local_name: "_bar2".into(),
    // };
    // assert_eq!(left, right);
    // let left = m.imports.get("baz").unwrap();
    // let right = &ImportDesc {
    //   source: "./baz".into(),
    //   name: "*".into(),
    //   local_name: "baz".into(),
    // };
    // assert_eq!(left, right);
  }
}
