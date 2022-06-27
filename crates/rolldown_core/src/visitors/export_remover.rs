use ast::{Decl, ExportDecl, Id, ModuleItem, Stmt};
use hashbrown::HashSet;
use swc_common::DUMMY_SP;
use swc_ecma_visit::Fold;

pub struct ExportRemover {
    pub unused_ids: HashSet<Id>,
}

impl Fold for ExportRemover {
    fn fold_module_item(&mut self, n: ast::ModuleItem) -> ast::ModuleItem {
        match n {
            ast::ModuleItem::ModuleDecl(ast::ModuleDecl::ExportDecl(decl)) => match decl.decl {
                ast::Decl::Class(class) => {
                    if self.unused_ids.contains(&class.ident.to_id()) {
                        ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Class(class)))
                    } else {
                        ModuleItem::ModuleDecl(ast::ModuleDecl::ExportDecl(ExportDecl {
                            decl: Decl::Class(class),
                            span: DUMMY_SP,
                        }))
                    }
                }
                ast::Decl::Fn(func) => {
                    tracing::debug!("func.ident.to_id() {:?}", func.ident.to_id());
                    if self.unused_ids.contains(&func.ident.to_id()) {
                        ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Fn(func)))
                    } else {
                        ModuleItem::ModuleDecl(ast::ModuleDecl::ExportDecl(ExportDecl {
                            decl: Decl::Fn(func),
                            span: DUMMY_SP,
                        }))
                    }
                }
                // ast::Decl::Var(var) => {
                //   if self.unused_ids.contains(&var.ident.to_id()) {
                //     ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Fn(func)))
                // } else {
                //     ModuleItem::ModuleDecl(ast::ModuleDecl::ExportDecl(ExportDecl {
                //         decl: Decl::Fn(func),
                //         span: DUMMY_SP,
                //     }))
                // }
                // },
                _ => ModuleItem::ModuleDecl(ast::ModuleDecl::ExportDecl(decl)),
            },
            _ => n,
        }
    }

    // fn fold_stmt(&mut self, n: Stmt) -> Stmt {
    //     n
    // }

    // fn fold_stmts(&mut self, n: Vec<Stmt>) -> Vec<Stmt> {
    //     n
    // }
}
