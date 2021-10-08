pub mod analyse;
pub mod scope;

pub mod helper {
  use swc_ecma_ast::{
    ClassDecl, Decl, DefaultDecl, ExportDefaultDecl, FnDecl, ModuleDecl, ModuleItem, Stmt,
  };

  // TODO: maybe we should make it as a method on ModuleItem not a plain function.
  // remove `export` from `export class Foo {...}`
  pub fn fold_export_decl_to_decl(mut module_item: ModuleItem) -> ModuleItem {
    if let ModuleItem::ModuleDecl(module_decl) = module_item {
      match module_decl {
        ModuleDecl::ExportDecl(export_decl) => ModuleItem::Stmt(Stmt::Decl(export_decl.decl)),
        // remove `export` from `export default class Foo {...}`
        ModuleDecl::ExportDefaultDecl(export_decl) => {
          if let DefaultDecl::Class(node) = export_decl.decl {
            ModuleItem::Stmt(Stmt::Decl(Decl::Class(ClassDecl {
              // TODO: fix case like `export default class {}`
              ident: node.ident.unwrap(),
              declare: false,
              class: node.class,
            })))
          } else if let DefaultDecl::Fn(node) = export_decl.decl {
            // TODO: fix case like `export default function {}`
            ModuleItem::Stmt(Stmt::Decl(Decl::Fn(FnDecl {
              ident: node.ident.unwrap(),
              declare: false,
              function: node.function,
            })))
          } else {
            ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(export_decl))
          }
        }
        _ => ModuleItem::ModuleDecl(module_decl),
      }
    } else {
      module_item
    }
  }
}
