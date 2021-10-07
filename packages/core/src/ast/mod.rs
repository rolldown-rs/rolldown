pub mod analyse;
pub mod scope;

pub mod helper {
  use swc_ecma_ast::{Decl, ModuleDecl, ModuleItem, Stmt};

  // remove `export` from `export class Foo {...}`
  // TODO: maybe we should make it as a method on ModuleItem not a plain function.
  pub fn fold_export_decl_to_decl(mut module_item: ModuleItem) -> ModuleItem {
    if let ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) = module_item {
      ModuleItem::Stmt(Stmt::Decl(export_decl.decl))
    } else {
      module_item
    }
  }
}