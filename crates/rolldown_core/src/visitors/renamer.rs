use std::sync::Mutex;

use ast::{ExportNamedSpecifier, Id, Ident};
use hashbrown::HashMap;
use swc_atoms::JsWord;
use swc_common::DUMMY_SP;
use swc_ecma_visit::{VisitMut, VisitMutWith};

use crate::ufriend::UFriend;

#[derive(Debug)]
pub struct Renamer<'a> {
    pub uf: &'a Mutex<UFriend<Id>>,
    pub rename_map: &'a HashMap<Id, JsWord>,
}

impl<'a> VisitMut for Renamer<'a> {
    fn visit_mut_ident(&mut self, ident: &mut Ident) {
        if let Some(root_id) = self.uf.lock().unwrap().find_root(&ident.to_id()) {
            if let Some(name) = self.rename_map.get(&root_id) {
                *ident = Ident::new(name.clone(), DUMMY_SP);
            }
        }
    }

    fn visit_mut_export_named_specifier(&mut self, node: &mut ExportNamedSpecifier) {
        node.visit_mut_children_with(self);
        let is_eq = node
            .exported
            .as_ref()
            .and_then(|module_export_name| match module_export_name {
                ast::ModuleExportName::Ident(ident) => Some(&ident.sym),
                _ => None,
            })
            .map(|exported| match &node.orig {
                ast::ModuleExportName::Ident(ident) => &ident.sym == exported,
                _ => false,
            })
            .unwrap_or_default();
        if is_eq {
            node.exported = None;
        }
    }
}
