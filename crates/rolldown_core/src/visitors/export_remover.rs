use ast::{
    ClassDecl, Decl, ExportDecl, FnDecl, Id, ModuleDecl, ModuleItem, Stmt, VarDecl, VarDeclKind,
    VarDeclarator,
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
    }
}

pub struct ExportRemover;

impl Fold for ExportRemover {
    fn fold_module_items(&mut self, items: Vec<ast::ModuleItem>) -> Vec<ast::ModuleItem> {
        items
            .into_iter()
            .flat_map(|module_item| match module_item {
                ModuleItem::ModuleDecl(ast::ModuleDecl::ExportDecl(decl)) => match decl.decl {
                    ast::Decl::Class(class) => {
                        vec![ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Class(class)))]
                    }
                    ast::Decl::Fn(fn_decl) => {
                        vec![ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Fn(fn_decl)))]
                    }
                    ast::Decl::Var(var) => {
                        vec![ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Var(var)))]
                    }
                    _ => unreachable!(),
                },
                ModuleItem::ModuleDecl(ast::ModuleDecl::ExportDefaultDecl(decl)) => match decl.decl
                {
                    ast::DefaultDecl::Class(cls) => {
                        vec![ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Class(
                            ClassDecl {
                                ident: cls.ident.unwrap(),
                                declare: false,
                                class: cls.class,
                            },
                        )))]
                    }
                    ast::DefaultDecl::Fn(func) => {
                        vec![ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Fn(FnDecl {
                            ident: func.ident.unwrap(),
                            declare: false,
                            function: func.function,
                        })))]
                    }
                    ast::DefaultDecl::TsInterfaceDecl(_) => unreachable!(),
                },
                // ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(_)) => vec![],
                _ => vec![module_item],
            })
            .collect()
    }
}
