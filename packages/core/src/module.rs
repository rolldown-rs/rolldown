use std::borrow::{Borrow, BorrowMut};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use swc_ecma_ast::{Decl, ImportDecl, ModuleDecl, ModuleItem, Stmt};

use crate::bundle::Bundle;
use crate::statement::Statement;
use crate::{helper, statement};
use crate::new_type::shared::{self, Shared};

pub struct ModuleOptions {
  pub id: String,
  pub source: String,
  pub bundle: Shared<Bundle>,
}

#[derive(Debug, Clone)]
pub struct Module {
  pub source: String,
  pub bundle: Shared<Bundle>,
  pub id: String,
  pub suggested_names: HashMap<String, String>,
  pub comments: Vec<ModuleDecl>,
  pub statements: Vec<Shared<Statement>>,
  pub import_declarations: Vec<Shared<Statement>>,
  pub export_declarations: Vec<Shared<Statement>>,
  pub imports: HashMap<String, ImportDesc>,
  // exports: HashMap<String, ImportDesc>,
  // pub module_decls: Vec<ModuleDecl>,
}
impl Module {
  pub fn new(op: ModuleOptions) -> Shared<Module> {
    let mut module = Module {
      source: op.source.clone(),
      bundle: op.bundle,
      id: op.id,
      suggested_names: HashMap::new(),
      comments: vec![],
      statements: vec![],
      import_declarations: vec![],
      export_declarations: vec![],
      imports: HashMap::new(),
    };
    let mut index = -1;
    let module = Shared::new(module);
    let ast = helper::parse_to_ast(op.source.clone());
    ast.body
      .into_iter()
      .for_each(|node| {
        index += 1;
        let statement = Shared::new(Statement::new(node, module.clone(), index));
        module.borrow_mut().statements.push(statement);
      });
    module.borrow_mut().import_declarations = module.borrow_mut().statements.clone().iter().map(|s| s.clone()).filter(|s| s.is_import_declaration).collect();
    module.borrow_mut().export_declarations = module.borrow_mut().statements.iter().map(|s| s.clone()).filter(|s| s.is_export_declaration).collect();
    module.borrow_mut().analyse();
    module
  }

  pub fn analyse(&mut self) {
    let mut imports = HashMap::new();
    // let mut exports = HashMap::new();
    // analyse imports and exports
    self
      .import_declarations
      .iter()
      .map(|s| &s.node)
      .for_each(|module_item| {
        if let ModuleItem::ModuleDecl(module_decl) = module_item {
          match module_decl {
            ModuleDecl::Import(import_decl) => {
              import_decl
                .specifiers
                .as_slice()
                .iter()
                .for_each(|specifier| {
                  let local_name;
                  let name;
                  match specifier {
                    swc_ecma_ast::ImportSpecifier::Default(n) => {
                      local_name = n.local.sym.to_string();
                      name = "default".to_owned();
                    }
                    swc_ecma_ast::ImportSpecifier::Named(n) => {
                      local_name = n.local.sym.to_string();
                      // There is inconsistency for acron and swc with `import { bar, bar2 as _bar2 } from './bar'`
                      // For `bar` , swc#local is bar, swc#imported is `None` ， acron#local and acron#imported both are bar2
                      // For `bar2` as `_bar2` , swc#local is _bar2, swc#imported is bar2 ， acron#local is _bar2 acron#imported is bar2
                      name = n.imported.as_ref().map_or(
                        local_name.clone(),            // for case `import { foo } from './foo'``
                        |ident| ident.sym.to_string(), // for case `import { foo as _foo } from './foo'``
                      );
                    }
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
            _ => {}
          }
        }
          
      });
    
    self.imports = imports;
    
    // TODO: exports

  }

  fn expand_all_statements(&self, is_entry_module: bool) -> Vec<&ModuleItem> {
    // let mut all_statements = vec![];
    // self.
    //   ast
    //   .iter()
    //   .for_each(|item| {
    //     match item {
    //       ModuleItem::ModuleDecl(_) => {},
    //       ModuleItem::Stmt(_) => {
    //         all_statements.push(item);
    //       }
    //     }
    //   });
    //   all_statements
    todo!()
  }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ImportDesc {
  source: String,
  name: String,
  local_name: String,
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn imports_test() {
    let fake_bundle = Bundle::new("./fake");
    let code = r#"
      import foo from './foo'
      import { bar, bar2 as _bar2 } from './bar'
      import * as baz from './baz'
    "#
    .to_owned();
    let fake_path = "./fake".to_owned();
    let m = Module::new(ModuleOptions {
      source: code.clone(),
      id: fake_path.to_owned(),
      bundle: Shared::new(fake_bundle),
    });

    let left = m.imports.get("foo").unwrap();
    let right = &ImportDesc {
      source: "./foo".into(),
      name: "default".into(),
      local_name: "foo".into(),
    };
    assert_eq!(left, right);
    let left = m.imports.get("bar").unwrap();
    let right = &ImportDesc {
      source: "./bar".into(),
      name: "bar".into(),
      local_name: "bar".into(),
    };
    assert_eq!(left, right);
    let left = m.imports.get("_bar2").unwrap();
    let right = &ImportDesc {
      source: "./bar".into(),
      name: "bar2".into(),
      local_name: "_bar2".into(),
    };
    assert_eq!(left, right);
    let left = m.imports.get("baz").unwrap();
    let right = &ImportDesc {
      source: "./baz".into(),
      name: "*".into(),
      local_name: "baz".into(),
    };
    assert_eq!(left, right);
  }
}
