use ast::{CallExpr, Callee, ExportSpecifier, Expr, Id, Lit, ModuleDecl, ModuleItem};
use hashbrown::{HashMap, HashSet};
use linked_hash_set::LinkedHashSet;
use swc_atoms::JsWord;
use swc_common;
use swc_ecma_visit::{noop_visit_mut_type, VisitMut, VisitMutWith};
mod export_remover;
pub use export_remover::*;

use crate::{
    collect_ident_of_pat, collect_js_word_of_pat, side_effect_of_module_item, LocalExports,
    MergedExports, SideEffect,
};

#[derive(Default)]
pub struct DependencyScanner {
    pub dependencies: LinkedHashSet<JsWord>,
    pub dyn_dependencies: HashSet<JsWord>,
    pub imports: HashMap<JsWord, HashSet<Specifier>>,
    pub re_exports: HashMap<JsWord, HashSet<Specifier>>,
    pub local_exports: LocalExports,
    pub merged_exports: MergedExports,
    pub side_effect: Option<SideEffect>,
}

impl DependencyScanner {
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
                        let alias = if s.imported.is_some() {
                            Some(s.local.sym.clone())
                        } else {
                            None
                        };
                        imports.insert(Specifier {
                            alias,
                            orginal: original,
                        });
                    }
                    ast::ImportSpecifier::Default(s) => {
                        imports.insert(Specifier {
                            alias: Some(s.local.sym.clone()),
                            orginal: "default".into(),
                        });
                    }
                    ast::ImportSpecifier::Namespace(s) => {
                        imports.insert(Specifier {
                            alias: Some(s.local.sym.clone()),
                            orginal: "*".into(),
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
                                re_exports.insert(Specifier {
                                    alias,
                                    orginal: ident_of_module_export_name(&s.orig).sym.clone(),
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
                                    alias: Some(ident_of_module_export_name(&s.name).sym.clone()),
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
                }
                ast::Decl::Fn(decl) => {
                    self.local_exports
                        .insert(decl.ident.sym.clone(), decl.ident.to_id());
                }
                ast::Decl::Var(decl) => {
                    decl.decls.iter().for_each(|decl| {
                        collect_ident_of_pat(&decl.name)
                            .into_iter()
                            .for_each(|ident| {
                                self.local_exports.insert(ident.sym.clone(), ident.to_id());
                            });
                    });
                }
                ast::Decl::TsInterface(_) => todo!(),
                ast::Decl::TsTypeAlias(_) => todo!(),
                ast::Decl::TsEnum(_) => todo!(),
                ast::Decl::TsModule(_) => todo!(),
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

impl VisitMut for DependencyScanner {
    noop_visit_mut_type!();

    fn visit_mut_module_item(&mut self, node: &mut ModuleItem) {
        if self.side_effect.is_none() {
            self.side_effect = side_effect_of_module_item(node)
        }
        node.visit_mut_children_with(self);
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
}

// pub type Specifier = ast::ImportSpecifier;
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Specifier {
    pub alias: Option<JsWord>,
    pub orginal: JsWord,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpecifierId {
    alias: Option<Id>,
    orginal: Id,
}

fn ident_of_module_export_name(name: &ast::ModuleExportName) -> ast::Ident {
    match name {
        ast::ModuleExportName::Ident(id) => id.clone(),
        ast::ModuleExportName::Str(_) => unreachable!(),
    }
}
