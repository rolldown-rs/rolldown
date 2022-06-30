use std::fmt::Debug;

use ast::{Id, ModuleItem};
use hashbrown::{HashMap, HashSet};
use linked_hash_set::LinkedHashSet;
use swc_atoms::JsWord;
use swc_common::{util::take::Take, Mark, Span};

use crate::{
    ufriend::UFriend, LocalExports, MergedExports, ModuleById, SideEffect, Specifier, SpecifierId, ResolvedId,
};

pub struct Module {
    pub exec_order: usize,
    pub id: JsWord,
    pub dependencies: LinkedHashSet<JsWord>,
    pub dyn_dependencies: HashSet<JsWord>,
    // source: String,
    pub ast: ast::Program,
    pub top_level_mark: Mark,
    pub imports: HashMap<JsWord, HashSet<SpecifierId>>,
    pub re_exports: HashMap<JsWord, HashSet<Specifier>>,
    pub local_exports: LocalExports,
    pub merged_exports: MergedExports,
    pub side_effect: Option<SideEffect>,
    pub resolved_module_ids: HashMap<JsWord, ResolvedId>,
    pub declared_ids: HashSet<Id>,
    pub included: bool,
    pub used_ids: HashSet<Id>,
    pub suggested_names: HashMap<JsWord, JsWord>,
    pub is_user_defined_entry: bool,
}

impl Module {
    pub fn suggest_name(&mut self, name: JsWord, suggested: JsWord) {
        self.suggested_names.insert(name, suggested);
    }

    pub fn depended_modules<'a>(&self, module_graph: &'a ModuleById) -> Vec<&'a Module> {
        self.dependencies
            .iter()
            .map(|unresolved_id| self.resolved_module_ids.get(unresolved_id).unwrap())
            .filter_map(|dep| module_graph.get(&dep.id))
            .collect()
    }

    pub fn dynamic_depended_modules<'a>(&self, module_graph: &'a ModuleById) -> Vec<&'a Module> {
        self.dyn_dependencies
            .iter()
            .map(|unresolved_id| self.resolved_module_ids.get(unresolved_id).unwrap())
            .filter_map(|dep| module_graph.get(&dep.id))
            .collect()
    }

    pub fn mark_used_id(&mut self, name: &JsWord, id: &Id, uf: &mut UFriend<Id>) {
        if name == "*" {
            // TODO: generate namespace export
        } else {
            uf.add_key(id.clone());
            let local_id = self
                .merged_exports
                .get(name)
                .unwrap_or_else(|| panic!("fail to get id {:?} in {:?}", name, self.id))
                .clone();
            uf.add_key(local_id.clone());
            uf.union(&id, &local_id);
            self.used_ids.insert(local_id);
        }
    }

    pub fn unused_ids(&self) -> HashSet<Id> {
        self.merged_exports
            .iter()
            .filter_map(|(name, id)| {
                if self.used_ids.contains(id) {
                    None
                } else {
                    Some(id.clone())
                }
            })
            .collect()
    }

    pub fn gen_export(&self) -> ast::ModuleItem {
        use ast::{
            ExportNamedSpecifier, ExportSpecifier, Ident, ModuleDecl, ModuleExportName, NamedExport,
        };
        use swc_common::{Span, DUMMY_SP};
        let mut exports = self.merged_exports.iter().collect::<Vec<_>>();
        exports.sort_by(|a, b| a.0.cmp(b.0));

        ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(NamedExport {
            span: Default::default(),
            specifiers: exports
                .into_iter()
                .map(|(name, id)| {
                    ExportSpecifier::Named(ExportNamedSpecifier {
                        span: Default::default(),
                        orig: ModuleExportName::Ident(ast::Ident {
                            sym: id.0.clone(),
                            span: Span {
                                ctxt: id.1,
                                ..DUMMY_SP
                            },
                            optional: false,
                        }),
                        exported: Some(ModuleExportName::Ident(Ident {
                            sym: name.clone(),
                            ..Ident::dummy()
                        })),
                        is_type_only: false,
                    })
                })
                .collect::<Vec<_>>(),
            src: None,
            type_only: false,
            asserts: None,
        }))
    }
}

impl Debug for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Module")
            .field("exec_order", &self.exec_order)
            .field("id", &self.id)
            .field("dependencies", &self.dependencies)
            .field("dyn_dependencies", &self.dyn_dependencies)
            .field("imports", &self.imports)
            .field("re_exports", &self.re_exports)
            .field("local_exports", &self.local_exports)
            .field("merged_exports", &self.merged_exports)
            .field("side_effect", &self.side_effect)
            .field("resolved_module_ids", &self.resolved_module_ids)
            .field("ast", &"...")
            .field("included", &self.included)
            .field("used_ids", &self.used_ids)
            .field("unused_ids", &self.unused_ids())
            .field("declared_ids", &self.declared_ids)
            .finish()
    }
}
