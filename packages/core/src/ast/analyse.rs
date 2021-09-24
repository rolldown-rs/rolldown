use std::borrow::Borrow;

use crate::{module::Module, types::shared::Shared};

// pub fn analyse(module: &mut Module) {
//   module
//     .statements
//     .iter_mut()
//     .for_each(|statement| {
//       statement
//         .borrow_mut()
//         .analyse();
//     })

// }

// struct ScopeAnalyser {
//   scope: Shared<Scope>,
//   new_scope: Option<Shared<Scope>>,
//   current_top_level_statement: Option<Statement>,
//   previous_statement: Option<Statement>,
// }
// impl ScopeAnalyser {
//   fn new(scope: Shared<Scope>) -> Self {
//     ScopeAnalyser {
//       scope,
//       new_scope: None,
//       current_top_level_statement: None,
//       previous_statement: None,
//     }
//   }
// }

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

// // impl ScopeAnalyser {

// // }

// impl Fold for ScopeAnalyser {

//   fn fold_fn_expr(&mut self, node: FnExpr) -> FnExpr {
//     // enter
//     let mut names = node.function.params.as_slice().iter().map(|p| {
//       if let Pat::Ident(binding_ident) = p.pat {
//         binding_ident.id.sym.to_string()
//       } else {
//         panic!("unsurrport function args")
//       }
//     })
//     .collect::<Vec<String>>();
//     if let Some(ident) = &node.ident {
//       names.push(ident.sym.to_string());
//     }

//     self
//       .new_scope
//       .replace(Shared::new(Scope::new(Some(self.scope.clone()), names, false)));

//     node.fold_children_with(self);

//     // leave

//     node
//   }
//   // fn visit_fn_decl(&mut self, node: &FnDecl, _parent: &dyn Node) {
//   //   let mut names = node.function.params.as_slice().iter().map(|p| {
//   //     if let Pat::Ident(binding_ident) = p.pat {
//   //       binding_ident.id.sym.to_string()
//   //     } else {
//   //       panic!("unsurrport function args")
//   //     }
//   //   })
//   //   .collect::<Vec<String>>();
//   //   let name = node.ident.sym.to_string();
//   //   names.push(name.clone());
//   //   add_to_scope(self, node)
//   // }
//   // fn visit_arrow_expr(&mut self, node: &ArrowExpr, _parent: &dyn Node) {

//   // }
// }
