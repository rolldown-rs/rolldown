use std::borrow::{Borrow, BorrowMut};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use swc_ecma_ast::{Decl, ImportDecl, ModuleDecl, ModuleItem, Stmt};

use crate::bundle::Bundle;
use crate::helper;

pub struct ModuleOptions {
  pub code: String,
  pub path: String,
  pub bundle: Rc<Bundle>,
}

#[derive(Debug)]
pub struct Module {
  pub code: String,
  pub path: String,
  pub bundle: Rc<Bundle>,
  pub suggested_names: HashMap<String, String>,
  pub imports: HashMap<String, ImportDesc>,
  // exports: HashMap<String, ImportDesc>,
  ast: Rc<RefCell<swc_ecma_ast::Module>>,
}
impl Module {
  pub fn new(op: ModuleOptions) -> Rc<Module> {
    let ast = helper::parse_to_ast(op.code.clone());
    // let mut collector = ImportsAndExportsCollector::new();
    // // let mut f = swc_ecma_visit::visit_module(&mut collector, ast, swc_ecma_ast::EmptyStmt);
    let mut m = Module {
      code: op.code,
      path: op.path,
      bundle: op.bundle,
      suggested_names: HashMap::new(),
      imports: HashMap::new(),
      // exports: HashMap::new(),
      ast,
    };
    m.analyse();
    Rc::new(m)
  }

  pub fn analyse(&mut self) {
    let mut imports = HashMap::new();
    // self.exports.clear();
    let ast = self.ast.as_ref().borrow();
    for module_item in ast.body.as_slice().iter() {
      if let ModuleItem::ModuleDecl(module_decl) = module_item {
        match module_decl {
          ModuleDecl::Import(import_decl) => {
            import_decl
              .specifiers
              .as_slice()
              .iter()
              .for_each(|specifier| {
                // let mut is_default = false;
                // let mut is_namespace = false;
                let local_name;
                let name;
                match specifier {
                  swc_ecma_ast::ImportSpecifier::Default(n) => {
                    // is_default = true;
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
                    // is_namespace = true;
                    local_name = n.local.sym.to_string();
                    name = "*".to_owned()
                  }
                }
                imports.insert(
                  local_name.clone(),
                  ImportDesc {
                    source: import_decl.src.value.to_string(),
                    local_name: local_name.clone(),
                    name,
                  },
                );
              });
          }
          _ => {}
        }
      }
    }
    self.imports = imports;
  }
}

#[derive(Debug, PartialEq)]
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
      code: code.clone(),
      path: fake_path.to_owned(),
      bundle: fake_bundle,
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

// struct ImportsAndExportsCollector {
//   imports: Vec<String>,
//   exports: Vec<String>,
// }
// impl ImportsAndExportsCollector {
//   fn new() -> Self {
//     ImportsAndExportsCollector {
//       imports: vec![],
//       exports: vec![],
//     }
//   }
// }
// impl swc_ecma_visit::VisitMut for ImportsAndExportsCollector {
//   fn visit_mut_import_decl(&mut self, n: &mut ImportDecl) {
//     // let n = n.fold_children_with(self);
//     // TODO: exports
//     let imports = n
//       .specifiers
//       .as_slice()
//       .iter()
//       .map(|specifier| {
//         // let mut is_default = false;
//         // let mut is_namespace = false;
//         let local_name;
//         let name;
//         match specifier {
//           swc_ecma_ast::ImportSpecifier::Default(n) => {
//             // is_default = true;
//             local_name = n.local.to_string();
//             name = "default".to_owned();
//           }
//           swc_ecma_ast::ImportSpecifier::Named(n) => {
//             local_name = n.local.to_string();
//             name = n.imported.as_ref().unwrap().to_string();
//           }
//           swc_ecma_ast::ImportSpecifier::Namespace(n) => {
//             // is_namespace = true;
//             local_name = n.local.to_string();
//             name = "*".to_owned()
//           }
//         }
//         ImportDesc {
//           source: "TODO: ".to_owned(),
//           local_name,
//           name,
//         }
//       })
//       .collect::<Vec<ImportDesc>>();
//   }
// }
