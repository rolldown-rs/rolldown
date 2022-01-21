use crate::scanner::rel::{ExportDesc, ImportDesc};
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
use swc_ecma_ast::{Decl, Ident, ModuleDecl, ModuleItem, Stmt};
use swc_ecma_parser::Syntax;
use swc_ecma_visit::{noop_visit_mut_type, VisitMut, VisitMutWith};

use self::renamer::Renamer;
use crate::graph::Ctxt;
use crate::utils::{fold_export_decl_to_decl, union_find::UnionFind};

pub mod renamer;

#[derive(Clone, PartialEq, Eq)]
pub struct Module {
  pub id: String,
  pub module_side_effects: bool,
  pub statements: Vec<Statement>,
  pub exports: HashMap<String, ExportDesc>,
  pub definitions: HashMap<JsWord, SyntaxContext>,

  pub is_included: bool,
  pub need_renamed: HashMap<JsWord, JsWord>,
  // suggested export names, such as "default" / "*" / other names
  pub suggested_names: HashMap<String, String>,
  pub scanner: Option<Scanner>,
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
      definitions: Default::default(),
      // modifications: Default::default(),
      exports: Default::default(),
      scanner: Default::default(),
      need_renamed: Default::default(),
      is_included: false,
      scope: Scope::new(ScopeKind::Fn, Mark::fresh(Mark::root())),
      suggested_names: Default::default(),
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

    // println!("ast {:#?}", ast);

    self.scope = scanner.get_cur_scope().clone();

    self.scope.declared_symbols.keys().for_each(|sym: &JsWord| {
      self.definitions.insert(
        sym.clone(),
        self.scope.declared_symbols.get(&sym).unwrap().clone(),
      );
    });

    let statements = ast
      .body
      .into_par_iter()
      .map(|node| Statement::new(node))
      .collect::<Vec<Statement>>();

    self.statements = statements;

    self.scanner = Some(scanner.clone());

    scanner
  }

  pub fn rename(
    &mut self,
    symbol_rel: &UnionFind<Ctxt>,
    canonical_names: &HashMap<SyntaxContext, JsWord>,
  ) {
    // FIXME: use par_iter later
    self.statements.iter_mut().for_each(|stmt| {
      let mut ctxt_jsword_mapping: HashMap<SyntaxContext, JsWord> = Default::default();

      self.scope.declared_symbols.keys().for_each(|key| {
        if let Some(&ctxt) = self.scope.declared_symbols.get(&key) {
          ctxt_jsword_mapping.insert(ctxt, key.clone());
        }
      });

      let mut renamer = Renamer {
        ctxt_mapping: &self.scope.declared_symbols,
        mapping: &self.need_renamed,
        ctxt_jsword_mapping,
        symbol_rel,
        canonical_names,
      };

      println!("{:#?}", renamer);
      stmt.node.visit_mut_with(&mut renamer);
    });
  }

  pub fn render(&self) -> Vec<Statement> {
    self
      .statements
      .iter()
      .filter_map(|s| {
        if s.is_included {
          let mut stmt = s.clone();
          println!("{:#?}", stmt);

          if !self.is_entry && s.is_export_declaration {
            let module = &mut stmt.node;

            if let ModuleItem::ModuleDecl(ModuleDecl::ExportAll(_)) = &module {
              return None;
            }

            fold_export_decl_to_decl(module)
          }

          Some(stmt)
        } else {
          None
        }
      })
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
  noop_visit_mut_type!();

  fn visit_mut_ident(&mut self, ident: &mut Ident) {
    ident.span.ctxt = SyntaxContext::empty();
  }
}
