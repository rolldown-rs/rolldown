use ast::{
    BindingIdent, CallExpr, Callee, ExportSpecifier, Expr, Id, Ident,
    ImportDecl, Lit, ModuleDecl, ModuleItem, Stmt,
};
use hashbrown::{HashMap, HashSet};
use linked_hash_set::LinkedHashSet;
use swc_atoms::JsWord;
use swc_common::{self, Mark, DUMMY_SP};

use swc_ecma_utils::quote_ident;
use swc_ecma_visit::{noop_visit_mut_type, Visit, VisitMut, VisitMutWith, VisitWith};
mod export_remover;
pub use export_remover::*;
mod renamer;
pub use renamer::*;

use crate::{
    side_effect_of_module_item, LocalExports,
    MergedExports, SideEffect,
};

#[derive(Default)]
pub struct Scanner {
    pub dependencies: LinkedHashSet<JsWord>,
    pub dyn_dependencies: HashSet<JsWord>,
    pub imports: HashMap<JsWord, HashSet<SpecifierId>>,
    pub re_exports: HashMap<JsWord, HashSet<Specifier>>,
    pub local_exports: LocalExports,
    pub merged_exports: MergedExports,
    pub side_effect: Option<SideEffect>,

    // Imported bindding is not included.
    pub declared_ids: HashSet<Id>,
    pub top_level_mark: swc_common::Mark,

    pub is_in_import: bool,
}

impl Scanner {
    fn add_dependency(&mut self, specifier: JsWord) {
        self.dependencies.insert_if_absent(specifier);
    }

    fn add_dynamic_import(&mut self, node: &CallExpr) {
        if let Callee::Import(_) = node.callee {
            if let Some(dyn_imported) = node.args.get(0) {
                if dyn_imported.spread.is_none() {
                    if let Expr::Lit(Lit::Str(imported)) = dyn_imported.expr.as_ref() {
                        self.dyn_dependencies.insert(imported.value.clone());
                    }
                }
            }
        }
    }

    fn add_import(&mut self, module_decl: &mut ModuleDecl) {
        if let ModuleDecl::Import(import_decl) = module_decl {
            let source = import_decl.src.value.clone();
            self.add_dependency(source.clone());
            let imports = self.imports.entry(source).or_default();
            import_decl
                .specifiers
                .iter()
                .for_each(|specifier| match specifier {
                    ast::ImportSpecifier::Named(s) => {
                        let original = s
                            .imported
                            .as_ref()
                            .map(|name| match name {
                                ast::ModuleExportName::Ident(id) => id.sym.clone(),
                                ast::ModuleExportName::Str(_) => todo!(),
                            })
                            .unwrap_or_else(|| s.local.sym.clone());
                        let alias = s.local.to_id();
                        imports.insert(SpecifierId { alias, original });
                    }
                    ast::ImportSpecifier::Default(s) => {
                        imports.insert(SpecifierId {
                            alias: s.local.to_id(),
                            original: "default".into(),
                        });
                    }
                    ast::ImportSpecifier::Namespace(s) => {
                        imports.insert(SpecifierId {
                            alias: s.local.to_id(),
                            original: "*".into(),
                        });
                    }
                });
        }
    }

    fn add_export(&mut self, module_decl: &ModuleDecl) -> Result<(), anyhow::Error> {
        match module_decl {
            ModuleDecl::ExportNamed(node) => {
                node.specifiers.iter().for_each(|specifier| {
                    match specifier {
                        ExportSpecifier::Named(s) => {
                            if let Some(source_node) = &node.src {
                                // export { name } from './other'
                                let source = source_node.value.clone();
                                self.add_dependency(source.clone());
                                let re_exports = self.re_exports.entry(source.clone()).or_default();
                                let alias = s
                                    .exported
                                    .as_ref()
                                    .map(|name| ident_of_module_export_name(name).sym.clone());
                                let orginal = ident_of_module_export_name(&s.orig).sym.clone();
                                re_exports.insert(Specifier {
                                    alias: alias.unwrap_or_else(|| orginal.clone()),
                                    orginal,
                                });
                            } else {
                                // export { name }
                                let ident = ident_of_module_export_name(&s.orig);
                                self.local_exports.insert(ident.sym.clone(), ident.to_id());
                            }
                        }
                        ExportSpecifier::Namespace(s) => {
                            // export * as name from './other'
                            let source = node.src.as_ref().map(|str| str.value.clone()).unwrap();
                            self.add_dependency(source.clone());
                            self.re_exports
                                .entry(source)
                                .or_default()
                                .insert(Specifier {
                                    alias: ident_of_module_export_name(&s.name).sym.clone(),
                                    orginal: "*".into(),
                                });
                        }
                        ExportSpecifier::Default(_) => {
                            // export v from 'mod';
                            // Rollup doesn't support it.
                        }
                    };
                });
            }
            ModuleDecl::ExportDecl(decl) => match &decl.decl {
                ast::Decl::Class(decl) => {
                    self.local_exports
                        .insert(decl.ident.sym.clone(), decl.ident.to_id());
                    self.declared_ids.insert(decl.ident.to_id());
                }
                ast::Decl::Fn(decl) => {
                    self.local_exports
                        .insert(decl.ident.sym.clone(), decl.ident.to_id());
                    self.declared_ids.insert(decl.ident.to_id());
                }
                ast::Decl::Var(decl) => {
                    let mut collector = VarDeclCollector::default();
                    decl.visit_with(&mut collector);
                    self.local_exports.extend(collector.local_exports.clone());
                    self.declared_ids.extend(
                        collector
                            .local_exports
                            .into_values()
                            .collect::<HashSet<_>>(),
                    );
                }
                ast::Decl::TsInterface(_) => todo!(),
                ast::Decl::TsTypeAlias(_) => todo!(),
                ast::Decl::TsEnum(_) => todo!(),
                ast::Decl::TsModule(_) => todo!(),
            },
            ModuleDecl::ExportDefaultDecl(node) => match &node.decl {
                ast::DefaultDecl::Class(cls) => {
                    self.local_exports.insert(
                        "default".into(),
                        cls.ident
                            .clone()
                            .unwrap_or_else(|| {
                                quote_ident!(DUMMY_SP.apply_mark(Mark::new()), "default")
                            })
                            .to_id(),
                    );
                    cls.ident.as_ref().map(|ident| {
                        self.declared_ids.insert(ident.to_id());
                    });
                }
                ast::DefaultDecl::Fn(func) => {
                    self.local_exports.insert(
                        "default".into(),
                        func.ident
                            .clone()
                            .unwrap_or_else(|| {
                                quote_ident!(DUMMY_SP.apply_mark(Mark::new()), "default")
                            })
                            .to_id(),
                    );
                    func.ident.as_ref().map(|ident| {
                        self.declared_ids.insert(ident.to_id());
                    });
                }
                ast::DefaultDecl::TsInterfaceDecl(_) => todo!(),
            },
            ModuleDecl::ExportDefaultExpr(node) => match node.expr.as_ref() {
                Expr::Ident(ident) => {
                    self.local_exports.insert("default".into(), ident.to_id());
                }
                _ => {
                    self.local_exports.insert(
                        "default".into(),
                        quote_ident!(DUMMY_SP.apply_mark(Mark::new()), "default").to_id(),
                    );
                }
            },
            ModuleDecl::ExportAll(node) => {
                // export * from './other'
                self.add_dependency(node.src.value.clone());
            }
            _ => {}
        }
        Ok(())
    }
}

impl VisitMut for Scanner {
    noop_visit_mut_type!();

    fn visit_mut_module_item(&mut self, node: &mut ModuleItem) {
        if self.side_effect.is_none() {
            self.side_effect = side_effect_of_module_item(node)
        }
        match node {
            ModuleItem::Stmt(Stmt::Decl(decl)) => match decl {
                ast::Decl::Class(decl) => {
                    self.declared_ids.insert(decl.ident.to_id());
                }
                ast::Decl::Fn(decl) => {
                    self.declared_ids.insert(decl.ident.to_id());
                }
                ast::Decl::Var(decl) => {
                    let mut collector = VarDeclCollector::default();
                    decl.visit_with(&mut collector);
                    self.declared_ids.extend(
                        collector
                            .local_exports
                            .into_values()
                            .collect::<HashSet<_>>(),
                    );
                }
                ast::Decl::TsInterface(_) => todo!(),
                ast::Decl::TsTypeAlias(_) => todo!(),
                ast::Decl::TsEnum(_) => todo!(),
                ast::Decl::TsModule(_) => todo!(),
            },
            _ => {}
        }
        node.visit_mut_children_with(self);
    }

    fn visit_mut_import_decl(&mut self, node: &mut ImportDecl) {
        self.is_in_import = true;
        node.visit_mut_children_with(self);
        self.is_in_import = false;
    }

    fn visit_mut_module_decl(&mut self, node: &mut ModuleDecl) {
        self.add_import(node);
        self.add_export(node).unwrap();
        node.visit_mut_children_with(self);
    }
    fn visit_mut_call_expr(&mut self, node: &mut CallExpr) {
        self.add_dynamic_import(node);
        node.visit_mut_children_with(self);
    }

    // fn visit_mut_binding_ident(&mut self, binding_ident: &mut BindingIdent) {
    //     if !self.is_in_import {
    //         let node = &binding_ident.id;
    //         let ident = Ident {
    //             sym: node.sym.clone(),
    //             span: DUMMY_SP.apply_mark(self.top_level_mark),
    //             optional: false,
    //         };
    //         if node.to_id() == ident.to_id() {
    //             self.declared_ids.insert(node.to_id());
    //         }
    //         binding_ident.visit_mut_children_with(self);
    //     }
    // }
}

// pub type Specifier = ast::ImportSpecifier;
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Specifier {
    pub alias: JsWord,
    pub orginal: JsWord,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpecifierId {
    pub alias: Id,
    pub original: JsWord,
}

fn ident_of_module_export_name(name: &ast::ModuleExportName) -> ast::Ident {
    match name {
        ast::ModuleExportName::Ident(id) => id.clone(),
        ast::ModuleExportName::Str(_) => unreachable!(),
    }
}

pub struct ClearMark;

impl VisitMut for ClearMark {
    noop_visit_mut_type!();
    fn visit_mut_ident(&mut self, node: &mut Ident) {
        node.span = DUMMY_SP;
    }
}

#[derive(Default)]
pub struct VarDeclCollector {
    pub local_exports: HashMap<JsWord, Id>,
}

impl Visit for VarDeclCollector {
    fn visit_binding_ident(&mut self, n: &BindingIdent) {
        let id = n.id.to_id();
        self.local_exports.insert(id.0.clone(), id);
    }

    fn visit_assign_pat_prop(&mut self, n: &ast::AssignPatProp) {
        let id = n.key.to_id();
        self.local_exports.insert(id.0.clone(), id);
    }
}
