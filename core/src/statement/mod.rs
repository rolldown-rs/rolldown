use std::{fmt::Debug, hash::Hash};

use swc_ecma_ast::*;

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct Statement {
  pub node: ModuleItem,
  pub is_import_declaration: bool,
  pub is_export_declaration: bool,
  pub is_included: bool,
}

impl Statement {
  pub fn new(node: ModuleItem) -> Self {
    let is_import_declaration = matches!(&node, ModuleItem::ModuleDecl(ModuleDecl::Import(_)));
    let is_export_declaration = if let ModuleItem::ModuleDecl(module_decl) = &node {
      matches!(
        module_decl,
        ModuleDecl::ExportAll(_)
          | ModuleDecl::ExportDecl(_)
          | ModuleDecl::ExportDefaultDecl(_)
          | ModuleDecl::ExportDefaultExpr(_)
          | ModuleDecl::ExportNamed(_)
      )
    } else {
      false
    };
    Statement {
      node,
      is_import_declaration,
      is_export_declaration,
      is_included: false,
    }
  }

  pub fn into_inner(self) -> ModuleItem {
    self.node
  }
}
