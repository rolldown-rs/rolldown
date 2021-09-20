use std::{borrow::{Borrow, BorrowMut}, cell::{RefCell, RefMut}, collections::HashMap };

use swc_ecma_ast::{Decl, Param, Pat, VarDeclKind};

use crate::{bundle::Bundle, new_type::shared::Shared};


#[derive(Debug, Clone)]
pub struct Scope {
  pub parnet: Option<Shared<Scope>>,
  pub depth: i32,
  pub declarations: HashMap<String, Shared<Decl>>,
  pub is_block_scope: bool,
}

impl Scope {
  pub fn new(parnet: Option<Shared<Scope>>, params: Option<Vec<Param>>, block: bool) -> Scope {
    let declarations = params.as_ref().map_or(HashMap::new(), |params| {
      let mut declarations = HashMap::new();
      params
        .iter()
        .for_each(|p| {
          if let Pat::Ident(binding_ident) = &p.pat {
            declarations.insert(binding_ident.id.sym.to_string(), params);
          }
        });
      declarations
    });
    Scope {
      depth: parnet.as_ref().map_or(0, |p| p.borrow().depth + 1),
      parnet,
      declarations: HashMap::new(),
      is_block_scope: block,
    }
  }

  fn add_declaration(&mut self, name: &str, declaration: &Shared<Decl>) {
		let is_block_declaration = if let Decl::Var(ref var_decl) = *declaration.borrow() {
      match var_decl.kind {
          VarDeclKind::Const => true,
          VarDeclKind::Let => true,
          _ => false,
      }
    } else {
      false
    };

    if !is_block_declaration && self.is_block_scope {
      self
        .parnet
        .as_ref()
        .unwrap()
        .borrow_mut()
        .add_declaration(name, declaration)
    } else {
      self.declarations.insert(name.to_owned(), declaration.clone());
    }
	}

  fn get_declaration (&self, name: &str) -> Option<&Shared<Decl>> {
    if self.declarations.contains_key(name) {
      return self.declarations.get(name);
    }
    if let Some(parent) = &self.parnet {
      parent.get_declaration(name)
    } else {
      None
    }
	}

	fn contains (&self, name: &str) -> bool {
    self
      .get_declaration(name)
      .is_some()
	}

	fn find_defining_scope (&self, name: &str) -> Option<&Self> {
    if self.declarations.contains_key(name) {
      return Some(self);
    }
    
    if let Some(parent) = &self.parnet {
      parent.find_defining_scope(name)
    } else {
      None
    }

	}
}