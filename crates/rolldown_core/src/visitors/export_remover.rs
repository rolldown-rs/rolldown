use ast::{
    ClassDecl, Decl, ExportDecl, FnDecl, Id, ModuleItem, Stmt, VarDecl, VarDeclKind, VarDeclarator,
};
use hashbrown::HashSet;
use swc_common::{util::take::Take, DUMMY_SP};
use swc_ecma_utils::quote_ident;
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
            .map(|module_item| match module_item {
                ModuleItem::ModuleDecl(ast::ModuleDecl::ExportDecl(decl)) => match decl.decl {
                    ast::Decl::Class(class) => {
                        ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Class(class)))
                    }
                    ast::Decl::Fn(fn_decl) => {
                        ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Fn(fn_decl)))
                    }
                    ast::Decl::Var(var) => ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Var(var))),
                    _ => unreachable!(),
                },
                ModuleItem::ModuleDecl(ast::ModuleDecl::ExportDefaultDecl(decl)) => match decl.decl
                {
                    ast::DefaultDecl::Class(cls) => {
                        ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Class(ClassDecl {
                            ident: cls.ident.unwrap(),
                            declare: false,
                            class: cls.class,
                        })))
                    }
                    ast::DefaultDecl::Fn(func) => {
                        ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Fn(FnDecl {
                            ident: func.ident.unwrap(),
                            declare: false,
                            function: func.function,
                        })))
                    }
                    ast::DefaultDecl::TsInterfaceDecl(_) => unreachable!(),
                },
                // ModuleItem::ModuleDecl(ast::ModuleDecl::ExportDefaultExpr(expr)) => {
                //     ModuleItem::Stmt(Stmt::Decl(Decl::Var(VarDecl {
                //         span: DUMMY_SP,
                //         declare: false,
                //         kind: VarDeclKind::Var,
                //         decls: vec![VarDeclarator {
                //             span: DUMMY_SP,
                //             name: ast::Pat::Ident(quote_ident!("_default").into()),
                //             init: Some(expr.expr),
                //             definite: false,
                //         }],
                //     })))
                // }
                _ => module_item,
            })
            .collect()
    }
}
