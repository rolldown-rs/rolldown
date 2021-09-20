use std::{borrow::Borrow, collections::HashMap};

use swc_ecma_ast::{FnExpr, ModuleDecl, ModuleItem, Pat};
use swc_ecma_visit::{Fold, FoldWith};

use crate::{ast::scope::Scope, module::Module, new_type::shared::Shared};

pub struct StatementOptions {
  
}

#[derive(Debug, Clone)]
pub struct Statement {
  pub node: ModuleItem,
  pub module: Shared<Module>,
  pub index: i32,
  pub id: String,
  pub scope: Scope,
  pub defines: HashMap<String, bool>,
  pub depends_on: HashMap<String, String>,
  pub strongly_depends_on: HashMap<String, String>,
  pub is_includeed: bool,
  pub is_import_declaration: bool,
  pub is_export_declaration: bool,
  // pub modifies: HashMap<String, String>,
  // pub included: bool,
  // source: String,
  // margin:           { value: [0, 0] },
}

impl Statement {
  pub fn new(node: ModuleItem, module: Shared<Module>, index: i32) -> Self {
    let is_import_declaration = if let ModuleItem::ModuleDecl(ModuleDecl::Import(_)) = &node {
      true
    } else {
      false
    };
    let is_export_declaration = !is_import_declaration;
    let id = module.id.clone() + "#" + &index.to_string();
    Statement {
      node,
      module,
      index,
      id,
      scope: Scope::new(None, None, false),
      defines: HashMap::new(),
      depends_on: HashMap::new(),
      strongly_depends_on: HashMap::new(),
      is_includeed: false,
      is_import_declaration,
      is_export_declaration,
    }
  }
}

impl Statement {
  pub fn analyse(&self) {
    if self.is_import_declaration { return }

    let statement = self;
    let scope = &self.scope;


  }
}


struct StatementAnalyser {
  scope: Shared<Scope>,
  // new_scope: Option<Shared<Scope>>,
}

impl StatementAnalyser {
  fn new(scope: Shared<Scope>) -> Self {
    StatementAnalyser {
      scope,
      // new_scope: None,
    }
  }
  fn test() {

  }
}

impl Fold for StatementAnalyser {
  fn fold_fn_expr(&mut self, node: FnExpr) -> FnExpr {
    // enter ---
    let mut new_scope = None;
    
    // TODO: should pass `node.function.params` to Scope, but it cause value moved.
    new_scope = Some(Shared::new(Scope::new(Some(self.scope.clone()), None, false)));
    // if let Some(ident) = node.ident {
    //   // TODO:
    //   // named function expressions - the name is considered
    //   // part of the function's scope
    //   new_scope.add_declaration(ident.sym.to_string(), declaration)
    // }

    if let Some(ref new_scope) = new_scope {
      self.scope = new_scope.clone()
    }
    // enter --- end
    let return_node = node.fold_children_with(self);
    // leave
    self.scope = new_scope.as_ref().unwrap().clone();
    // leave --- end
    return_node
  }
  // fn visit_fn_decl(&mut self, node: &FnDecl, _parent: &dyn Node) {
  //   let mut names = node.function.params.as_slice().iter().map(|p| {
  //     if let Pat::Ident(binding_ident) = p.pat {
  //       binding_ident.id.sym.to_string()
  //     } else {
  //       panic!("unsurrport function args")
  //     }
  //   })
  //   .collect::<Vec<String>>();
  //   let name = node.ident.sym.to_string();
  //   names.push(name.clone());
  //   add_to_scope(self, node)
  // }
  // fn visit_arrow_expr(&mut self, node: &ArrowExpr, _parent: &dyn Node) {

  // }
}


// fn add_to_scope(scope_analyser: &mut ScopeAnalyser, decl: &Decl) {
//   let name;
//   let scope = &scope_analyser.scope;
//   match decl {
//       Decl::Fn(fn_decl) => {
//         name = fn_decl.ident.to_string();
//       }
//       _ => panic!("unexpected decl")
//     }

//   Scope::add(scope, name.clone(), false);
//   if scope.parnet.is_some() {
//     scope_analyser
//       .current_top_level_statement
//       .borrow_mut()
//       .as_mut()
//       .expect("current_top_level_statement is none")
//       .defines
//       .insert(name.clone(), true);
//   }
// }

// fn add_to_block_scope(scope_analyser: &mut ScopeAnalyser, decl: &Decl) {
//   let name;
//   let scope = &scope_analyser.scope;
//   match decl {
//       Decl::Fn(fn_decl) => {
//         name = fn_decl.ident.to_string();
//       }
//       _ => panic!("unexpected decl")
//     }

//   Scope::add(scope, name.clone(), true);
//   if scope.parnet.is_some() {
//     scope_analyser
//       .current_top_level_statement
//       .borrow_mut()
//       .as_mut()
//       .expect("current_top_level_statement is none")
//       .defines
//       .insert(name.clone(), true);
//   }
// }

