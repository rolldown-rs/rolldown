pub mod ast_sugar;
mod lcp;
pub mod name_helpers;
pub mod side_effect;
pub use lcp::*;
use std::path::Path;

use swc_ecma_ast::{EsVersion, ExportSpecifier, ImportSpecifier, ModuleDecl, ModuleItem};

use swc_common::{
  errors::{ColorConfig, Handler},
  FileName,
};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};
use swc_ecma_parser::{EsConfig, TsConfig};

mod hook;
mod statement;
pub use hook::*;
pub use statement::*;

use crate::compiler::SOURCE_MAP;

pub mod path {
  pub fn relative_id(id: String) -> String {
    if nodejs_path::is_absolute(&id) {
      nodejs_path::relative(&nodejs_path::resolve!("."), &id)
    } else {
      id
    }
  }
}

#[inline]
pub fn is_external_module(source: &str) -> bool {
  source.starts_with("node:") || (!nodejs_path::is_absolute(source) && !source.starts_with('.'))
}

#[inline]
pub fn is_decl_or_stmt(node: &ModuleItem) -> bool {
  matches!(
    node,
    ModuleItem::ModuleDecl(
      ModuleDecl::ExportDecl(_)
        | ModuleDecl::ExportDefaultExpr(_)
        | ModuleDecl::ExportDefaultDecl(_)
    ) | ModuleItem::Stmt(_)
  )
}

#[inline]
pub fn is_import_namespace(node: &ModuleItem) -> bool {
  if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = &node {
    if let Some(ImportSpecifier::Namespace(_)) = import_decl.specifiers.get(0) {
      return true;
    }
  }
  false
}

#[inline]
pub fn is_export_namespace(node: &ModuleItem) -> bool {
  if let ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(named_export)) = &node {
    // FIXME: split `export * as foo, { foo } from "./foo"`
    if let Some(ExportSpecifier::Namespace(_)) = named_export.specifiers.get(0) {
      return true;
    }
  }
  false
}

pub fn parse_file(source_code: String, filename: &str) -> swc_ecma_ast::Module {
  let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(SOURCE_MAP.clone()));
  let p = Path::new(filename);
  let fm = SOURCE_MAP.new_source_file(FileName::Custom(filename.to_owned()), source_code);
  let ext = p.extension().and_then(|ext| ext.to_str()).unwrap_or("js");
  let syntax = if ext == "ts" || ext == "tsx" {
    Syntax::Typescript(TsConfig {
      decorators: false,
      tsx: ext == "tsx",
      ..Default::default()
    })
  } else {
    Syntax::Es(EsConfig {
      static_blocks: true,
      private_in_object: true,
      import_assertions: true,
      jsx: ext == "jsx",
      export_default_from: true,
      decorators_before_export: true,
      decorators: true,
      fn_bind: true,
      allow_super_outside_method: true,
    })
  };

  let lexer = Lexer::new(
    syntax,
    EsVersion::latest(),
    StringInput::from(fm.as_ref()),
    None,
  );
  let mut parser = Parser::new_from(lexer);
  parser.take_errors().into_iter().for_each(|e| {
    e.into_diagnostic(&handler).emit();
  });
  parser.parse_module().unwrap()
}
