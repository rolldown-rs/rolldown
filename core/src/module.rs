use crate::ast;
use crate::plugin_driver::PluginDriver;
use crate::symbol_box::SymbolBox;
use crate::utils::resolve_id;
use std::collections::HashMap;

use std::sync::Mutex;
use std::{collections::HashSet, hash::Hash};

use ast::{ModuleDecl, ModuleItem};
use swc_atoms::JsWord;

use swc_common::util::take::Take;
use swc_common::{Mark, SyntaxContext};
use swc_ecma_ast::Ident;

use swc_ecma_visit::{noop_visit_mut_type, VisitMut};

use crate::scanner::rel::{ExportDesc, ReExportDesc};
use crate::types::ResolvedId;

#[derive(Clone, PartialEq, Eq)]
pub struct Module {
  pub ast: ast::Module,
  pub id: String,
  pub local_exports: HashMap<JsWord, ExportDesc>,
  pub re_exports: HashMap<JsWord, ReExportDesc>,
  pub re_export_all_sources: HashSet<JsWord>,
  pub exports: HashMap<JsWord, Mark>,
  pub declared: HashMap<JsWord, Mark>,
  pub resolved_ids: HashMap<JsWord, ResolvedId>,
  pub suggested_names: HashMap<JsWord, JsWord>,
}

impl Module {
  pub fn new(id: String) -> Self {
    Self {
      ast: ast::Module::dummy(),
      id,
      local_exports: Default::default(),
      re_export_all_sources: Default::default(),
      re_exports: Default::default(),
      exports: Default::default(),
      declared: Default::default(),
      resolved_ids: Default::default(),
      suggested_names: Default::default(),
    }
  }

  pub fn link_local_exports(&mut self) {
    self.local_exports.iter().for_each(|(key, info)| {
      self.exports.insert(key.clone(), info.mark);
    });
    self.re_exports.iter().for_each(|(key, info)| {
      self.exports.insert(key.clone(), info.mark);
    });
    // We couldn't deal with `export * from './foo'` now.
  }

  pub fn bind_local_references(&self, symbol_box: &mut SymbolBox) {
    self.local_exports.iter().for_each(|(name, export_desc)| {
      let name = if let Some(default_exported_ident) = &export_desc.identifier {
        default_exported_ident
      } else {
        name
      };
      if name == "default" {
        // This means that the module's `export default` is a value. No name to bind.
        // And we need to generate a name for it lately.
        return;
      }
      if let Some(declared_name_mark) = self.declared.get(name) {
        symbol_box.union(export_desc.mark, *declared_name_mark);
      } else {
        panic!("unkown export {:?} for module {}", name, self.id);
      }
    });
  }

  pub fn include(&mut self) {
    self
      .ast
      .body
      .retain(|module_item| !matches!(module_item, ModuleItem::ModuleDecl(ModuleDecl::Import(_))));
  }

  pub fn suggest_name(&mut self, name: JsWord, suggested: JsWord) {
    self.suggested_names.insert(name, suggested);
  }

  pub fn resolve_id(
    &mut self,
    dep_src: &JsWord,
    plugin_driver: &Mutex<PluginDriver>,
  ) -> ResolvedId {
    self
      .resolved_ids
      .entry(dep_src.clone())
      .or_insert_with_key(|key| {
        resolve_id(key, Some(&self.id), false, &plugin_driver.lock().unwrap())
      })
      .clone()
  }
}

impl std::fmt::Debug for Module {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Module")
      .field("id", &self.id)
      .field("local_exports", &self.local_exports)
      .field("re_exports", &self.re_exports)
      .field("re_export_all_sources", &self.re_export_all_sources)
      .field("exports", &self.exports)
      .field("declared", &self.declared)
      .field("resolved_ids", &self.resolved_ids)
      .field("suggested_names", &self.suggested_names)
      .finish()
  }
}

impl Hash for Module {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    state.write(&self.id.as_bytes());
  }
}

#[derive(Clone, Copy)]
struct ClearMark;
impl VisitMut for ClearMark {
  noop_visit_mut_type!();

  fn visit_mut_ident(&mut self, ident: &mut Ident) {
    ident.span.ctxt = SyntaxContext::empty();
  }
}
