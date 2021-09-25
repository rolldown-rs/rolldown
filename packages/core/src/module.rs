use std::borrow::{Borrow, BorrowMut};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use swc_ecma_ast::{
  Decl, DefaultDecl, ExportDefaultDecl, ExportSpecifier, ImportDecl, ModuleDecl, ModuleItem, Stmt,
};

use crate::ast::analyse;
use crate::bundle::Bundle;
use crate::graph::{Graph, ModOrExt};
use crate::statement::Statement;
use crate::types::shared::{self, Shared};
use crate::{graph, helper, statement};
use swc_common::{
  errors::{ColorConfig, Handler},
  sync::Lrc,
  FileName, FilePathMapping, SourceMap,
};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};

pub struct ModuleOptions {
  pub id: String,
  pub source: String,
  pub graph: Shared<Graph>,
}

#[derive(Clone)]
pub struct Module {
  // pub cm: Lrc<SourceMap>,
  pub source: String,
  pub graph: Shared<Graph>,
  pub id: String,
  pub statements: Vec<Shared<Statement>>,
  // pub import_declarations: Vec<Statement>,
  // pub export_declarations: Vec<Statement>,
  pub imports: HashMap<String, ImportDesc>,
  pub exports: HashMap<String, ExportDesc>,
}
impl Module {
  pub fn new(op: ModuleOptions) -> Self {
    let mut module = Module {
      graph: op.graph,
      source: op.source,
      // bundle: op.bundle,
      id: op.id,
      // suggested_names: HashMap::new(),
      // comments: vec![],
      statements: vec![],
      // import_declarations: vec![],
      // export_declarations: vec![],
      imports: HashMap::new(),
      exports: HashMap::new(),
      // definitions: HashMap::new(),
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
    let import_declarations = self
      .statements
      .clone()
      .iter()
      .map(|s| s.clone())
      .filter(|s| s.is_import_declaration)
      .collect::<Vec<Shared<Statement>>>();

    // analyse imports

    let mut imports = HashMap::new();
    // let mut exports = HashMap::new();
    // analyse imports and exports
    import_declarations
      .iter()
      .map(|s| &s.node)
      .for_each(|module_item| {
        if let ModuleItem::ModuleDecl(module_decl) = module_item {
          match module_decl {
            ModuleDecl::Import(import_decl) => {
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
                    // module: None,
                  },
                );
              });
            }
            _ => {}
          }
        }
      });

    self.imports = imports;
  }

  pub fn expand_all_statements(&mut self, is_entry_module: bool) -> Vec<Shared<Statement>> {
    let mut all_statements: Vec<Shared<Statement>> = vec![];
    self.statements.iter().for_each(|statement| {
      if statement.is_included {
        // let index = all_statements.iter().postion
        return;
      }

      if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = &statement.node {
        // import './effects'
        if import_decl.specifiers.len() == 0 {
        } else {
          let module = Graph::fetch_module(
            &self.graph,
            &import_decl.src.value.to_string(),
            Some(&self.id),
          );
          if let ModOrExt::Mod(ref m) = module {
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
    let handler =
      Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(self.graph.cm.clone()));
    let fm = self
      .graph
      .cm
      .new_source_file(FileName::Custom(self.id.clone()), self.source.clone());

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

    let module = parser
      .parse_module()
      .map_err(|mut e| {
        // Unrecoverable fatal error occurred
        e.into_diagnostic(&handler).emit()
      })
      .expect("failed to parser module");
    module
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
  use super::*;
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
