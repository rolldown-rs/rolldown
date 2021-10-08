pub mod analyse;
pub mod scope;

pub mod helper {
  use swc_ecma_ast::{ClassDecl, Decl, DefaultDecl, FnDecl, ModuleDecl, ModuleItem, Stmt};

  // TODO: maybe we should make it as a method on ModuleItem not a plain function.
  // remove `export` from `export class Foo {...}`
  pub fn fold_export_decl_to_decl(module_item: &mut ModuleItem) {
    if let ModuleItem::ModuleDecl(module_decl) = &module_item {
      *module_item = match module_decl {
        ModuleDecl::ExportDecl(export_decl) => {
          ModuleItem::Stmt(Stmt::Decl(export_decl.decl.clone()))
        }
        // remove `export` from `export default class Foo {...}`
        ModuleDecl::ExportDefaultDecl(export_decl) => {
          if let DefaultDecl::Class(node) = &export_decl.decl {
            ModuleItem::Stmt(Stmt::Decl(Decl::Class(ClassDecl {
              // TODO: fix case like `export default class {}`
              ident: node.ident.clone().unwrap(),
              declare: false,
              class: node.class.clone(),
            })))
          } else if let DefaultDecl::Fn(node) = &export_decl.decl {
            // TODO: fix case like `export default function {}`
            ModuleItem::Stmt(Stmt::Decl(Decl::Fn(FnDecl {
              ident: node.ident.clone().unwrap(),
              declare: false,
              function: node.clone().function,
            })))
          } else {
            ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(export_decl.clone()))
          }
        }
        _ => ModuleItem::ModuleDecl(module_decl.clone()),
      };
    }
  }
}
