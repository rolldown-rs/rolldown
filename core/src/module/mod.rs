use crate::scanner::rel::ExportDesc;
use crate::scanner::scope::{Scope, ScopeKind};
use crate::scanner::Scanner;
use crate::utils::parse_file;
use crate::{
  graph::{DepNode, SOURCE_MAP},
  statement::Statement,
};
use rayon::prelude::*;
use std::{collections::HashMap, hash::Hash};
use swc_atoms::JsWord;
use swc_common::{Mark, SyntaxContext};
use swc_ecma_ast::{Ident, ModuleItem};
use swc_ecma_visit::{VisitMut, VisitMutWith};

use self::renamer::Renamer;

pub mod renamer;

#[derive(Clone, PartialEq, Eq)]
pub struct Module {
  pub id: String,
  pub module_side_effects: bool,
  pub statements: Vec<Statement>,
  pub exports: HashMap<String, ExportDesc>,

  pub is_included: bool,
  pub need_renamed: HashMap<JsWord, JsWord>,
  pub scope: Scope,
  pub is_entry: bool,
}

impl std::fmt::Debug for Module {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Module")
      .field("id", &self.id)
      .field(
        "declared",
        &self
          .scope
          .declared_symbols
          .keys()
          .map(|s| s.to_string())
          .collect::<Vec<String>>(),
      )
      .field("need_renamed", &self.need_renamed)
      .field("scope", &self.scope)
      .finish()
  }
}

impl Into<DepNode> for Module {
  fn into(self) -> DepNode {
    DepNode::Mod(self)
  }
}

impl Module {
  pub fn new(id: String, is_entry: bool) -> Self {
    let mark = Mark::fresh(Mark::root());
    println!("mark1 {:?}", mark);
    let mark = Mark::fresh(Mark::root());
    println!("mark2 {:?}", mark);
    Module {
      id,
      module_side_effects: true,
      statements: Default::default(),
      // definitions: Default::default(),
      // modifications: Default::default(),
      exports: Default::default(),
      need_renamed: Default::default(),
      is_included: false,
      scope: Scope::new(ScopeKind::Fn, Mark::fresh(Mark::root())),
      is_entry,
    }
  }

  pub fn include_all(&mut self) {
    self.statements.par_iter_mut().for_each(|s| {
      s.is_included = true;
    });
    self.is_included = true;
  }

  pub fn set_source(&mut self, source: String) -> Scanner {
    let mut ast = parse_file(source, &self.id, &SOURCE_MAP).unwrap();

    ast.body.sort_by(|a, b| {
      use std::cmp::Ordering;
      let is_a_module_decl = matches!(a, ModuleItem::ModuleDecl(_));
      let is_b_module_decl = matches!(b, ModuleItem::ModuleDecl(_));
      if is_a_module_decl && !is_b_module_decl {
        Ordering::Less
      } else if is_b_module_decl && !is_a_module_decl {
        Ordering::Greater
      } else {
        Ordering::Equal
      }
    });

    ast.visit_mut_children_with(&mut ClearMark);

    let mut scanner = Scanner::new(self.scope.clone());

    ast.visit_mut_children_with(&mut scanner);

    println!("ast {:#?}", ast);

    self.scope = scanner.get_cur_scope().clone();

    let statements = ast
      .body
      .into_par_iter()
      .map(|node| Statement::new(node))
      .collect::<Vec<Statement>>();

    self.statements = statements;

    scanner
  }

  pub fn rename(&mut self) {
    self.statements.par_iter_mut().for_each(|stmt| {
      let mut renamer = Renamer {
        ctxt_mapping: &self.scope.declared_symbols,
        mapping: &self.need_renamed,
      };
      stmt.node.visit_mut_with(&mut renamer);
    });
  }

  pub fn render(&self) -> Vec<Statement> {
    self
      .statements
      .iter()
      .filter_map(|s| if s.is_included { Some(s.clone()) } else { None })
      .map(|mut stmt| {
        // fold_export_decl_to_decl(&mut stmt.node);
        stmt
      })
      .collect()
  }
}

impl Hash for Module {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    state.write(&self.id.as_bytes());
  }
}

#[derive(Clone, Copy)]
struct ClearMark;
impl VisitMut for ClearMark {
  fn visit_mut_ident(&mut self, ident: &mut Ident) {
    ident.span.ctxt = SyntaxContext::empty();
  }
}
