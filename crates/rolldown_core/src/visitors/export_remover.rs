use ast::{ClassDecl, Decl, ExportDecl, Id, ModuleItem, Stmt};
use hashbrown::HashSet;
use swc_common::{util::take::Take, DUMMY_SP};
use swc_ecma_visit::Fold;

pub struct TreeShakeExportRemover {
    pub unused_ids: HashSet<Id>,
}

impl Fold for TreeShakeExportRemover {
    fn fold_module_items(&mut self, items: Vec<ast::ModuleItem>) -> Vec<ast::ModuleItem> {
        items
            .into_iter()
            .flat_map(|mut module_item| match &mut module_item {
                ModuleItem::ModuleDecl(ast::ModuleDecl::ExportDecl(decl)) => match &mut decl.decl {
                    ast::Decl::Class(class) => {
                        if self.unused_ids.contains(&class.ident.to_id()) {
                            [ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Class(
                                std::mem::replace(
                                    class,
                                    ClassDecl {
                                        ident: Take::dummy(),
                                        declare: false,
                                        class: Take::dummy(),
                                    },
                                ),
                            )))]
                        } else {
                            [module_item]
                        }
                    }
                    _ => [module_item],
                },
                _ => [module_item],
            })
            .collect()
        // match n {
        //     ast::ModuleItem::ModuleDecl(ast::ModuleDecl::ExportDecl(decl)) => match decl.decl {
        //         ast::Decl::Class(class) => {
        //             if self.unused_ids.contains(&class.ident.to_id()) {
        //                 ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Class(class)))
        //             } else {
        //                 ModuleItem::ModuleDecl(ast::ModuleDecl::ExportDecl(ExportDecl {
        //                     decl: Decl::Class(class),
        //                     span: DUMMY_SP,
        //                 }))
        //             }
        //         }
        //         ast::Decl::Fn(func) => {
        //             tracing::debug!("func.ident.to_id() {:?}", func.ident.to_id());
        //             if self.unused_ids.contains(&func.ident.to_id()) {
        //                 ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Fn(func)))
        //             } else {
        //                 ModuleItem::ModuleDecl(ast::ModuleDecl::ExportDecl(ExportDecl {
        //                     decl: Decl::Fn(func),
        //                     span: DUMMY_SP,
        //                 }))
        //             }
        //         }
        //         _ => ModuleItem::ModuleDecl(ast::ModuleDecl::ExportDecl(decl)),
        //     },
        //     _ => n,
        // }
    }

    // fn fold_stmt(&mut self, n: Stmt) -> Stmt {
    //     n
    // }

    // fn fold_stmts(&mut self, n: Vec<Stmt>) -> Vec<Stmt> {
    //     n
    // }
}

pub struct ExportRemover;

impl Fold for ExportRemover {
    fn fold_module_items(&mut self, items: Vec<ast::ModuleItem>) -> Vec<ast::ModuleItem> {
        items
            .into_iter()
            .map(|mut module_item| match &mut module_item {
                ModuleItem::ModuleDecl(ast::ModuleDecl::ExportDecl(decl)) => match &mut decl.decl {
                    ast::Decl::Class(class) => {
                        ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Class(std::mem::replace(
                            class,
                            ClassDecl {
                                ident: Take::dummy(),
                                declare: false,
                                class: Take::dummy(),
                            },
                        ))))
                    }
                    ast::Decl::Fn(fn_decl) => {
                        ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Fn(std::mem::replace(
                            fn_decl,
                            ast::FnDecl {
                                ident: Take::dummy(),
                                declare: false,
                                function: Take::dummy(),
                            },
                        ))))
                    }
                    ast::Decl::Var(var) => {
                        ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Var(std::mem::replace(
                            var,
                            ast::VarDecl {
                                span: Take::dummy(),
                                declare: false,
                                decls: vec![],
                                kind: var.kind.clone(),
                            },
                        ))))
                    }
                    _ => module_item,
                },
                _ => module_item,
            })
            .collect()
    }
}
