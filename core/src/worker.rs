use std::sync::{Arc, Mutex};

use crossbeam::{channel::Sender, queue::SegQueue};
use dashmap::DashSet;
use node_resolve::resolve;

use swc_ecma_ast::{ModuleDecl, ModuleItem};
use swc_ecma_visit::VisitMutWith;

use crate::types::IsExternal;
use crate::{
  graph::{Msg, Rel},
  module::Module,
  plugin_driver::PluginDriver,
  scanner::{scope::BindType, Scanner},
  symbol_box::SymbolBox,
  types::ResolvedId,
  utils::{load, parse_file},
};

pub struct Worker {
  pub symbol_box: Arc<Mutex<SymbolBox>>,
  pub job_queue: Arc<SegQueue<ResolvedId>>,
  pub tx: Sender<Msg>,
  pub plugin_driver: Arc<Mutex<PluginDriver>>,
  pub processed_id: Arc<DashSet<String>>,
  pub external: Arc<Mutex<Vec<IsExternal>>>,
}

impl Worker {
  #[inline]
  fn fetch_job(&self) -> Option<ResolvedId> {
    self
      .job_queue
      .pop()
      .filter(|resolved_id| !self.processed_id.contains(&resolved_id.id))
      .map(|resolved_id| {
        self.processed_id.insert(resolved_id.id.clone());
        resolved_id
      })
  }

  #[inline]
  pub fn run(&mut self) {
    if let Some(resolved_id) = self.fetch_job() {
      if resolved_id.external {
      } else {
        let mut module = Module::new(resolved_id.id.clone());
        let source = load(&resolved_id.id, &self.plugin_driver.lock().unwrap());
        let mut ast = parse_file(source, &module.id);
        self.pre_analyze_imported_module(&mut module, &ast);

        let mut scanner = Scanner::new(self.symbol_box.clone(), self.tx.clone());
        ast.visit_mut_with(&mut scanner);

        scanner.import_infos.iter().for_each(|(imported, info)| {
          let resolved_id = module.resolve_id(imported, &self.plugin_driver);
          self
            .tx
            .send(Msg::DependencyReference(
              module.id.clone(),
              resolved_id.id,
              info.clone().into(),
            ))
            .unwrap();
        });
        scanner
          .re_export_infos
          .iter()
          .for_each(|(re_exported, info)| {
            let resolved_id = module.resolve_id(re_exported, &self.plugin_driver);
            self
              .tx
              .send(Msg::DependencyReference(
                module.id.clone(),
                resolved_id.id,
                info.clone().into(),
              ))
              .unwrap();
          });
        scanner.export_all_sources.iter().for_each(|re_exported| {
          let resolved_id = module.resolve_id(re_exported, &self.plugin_driver);
          self
            .tx
            .send(Msg::DependencyReference(
              module.id.clone(),
              resolved_id.id,
              Rel::ReExportAll,
            ))
            .unwrap();
        });

        module.local_exports = scanner.local_exports;
        module.re_exports = scanner.re_exports;
        module.re_export_all_sources = scanner.export_all_sources;
        // module.declared =
        {
          let root_scope = scanner.stacks.into_iter().next().unwrap();
          let declared_symbols = root_scope.declared_symbols;
          let mut declared_symbols_kind = root_scope.declared_symbols_kind;
          declared_symbols.into_iter().for_each(|(name, mark)| {
            let bind_type = declared_symbols_kind.remove(&name).unwrap();
            if BindType::Import == bind_type {
              module.imported_symbols.insert(name, mark);
            } else {
              module.declared_symbols.insert(name, mark);
            }
          });
        }
        module.namespace.mark = self.symbol_box.lock().unwrap().new_mark();

        module.ast = ast;

        module.bind_local_references(&mut self.symbol_box.lock().unwrap());

        module.link_local_exports();

        log::debug!("[worker]: emit module {:#?}", module);
        self.tx.send(Msg::NewMod(module)).unwrap();
      }
    }
  }

  // Fast path for analyzing static import and export.
  #[inline]
  pub fn pre_analyze_imported_module(&self, module: &mut Module, ast: &swc_ecma_ast::Module) {
    ast.body.iter().for_each(|module_item| {
      if let ModuleItem::ModuleDecl(module_decl) = module_item {
        let mut depended = None;
        match module_decl {
          ModuleDecl::Import(import_decl) => {
            depended = Some(&import_decl.src.value);
          }
          ModuleDecl::ExportNamed(node) => {
            if let Some(source_node) = &node.src {
              depended = Some(&source_node.value);
            }
          }
          ModuleDecl::ExportAll(node) => {
            depended = Some(&node.src.value);
          }
          _ => {}
        }
        if let Some(depended) = depended {
          let mut resolved_id = module.resolve_id(depended, &self.plugin_driver);
          let is_external =
            self
              .external
              .lock()
              .unwrap()
              .iter()
              .find_map(|test_func| -> Option<bool> {
                Some(test_func(
                  resolved_id.id.as_str(),
                  Some(module.id.as_str()),
                  false,
                ))
              });
          let internal_external = resolved_id.external;

          resolved_id.external = {
            if internal_external {
              true
            } else {
              // include all by default
              is_external.unwrap_or(false)
            }
          };

          self.job_queue.push(resolved_id);
        }
      }
    });
  }
}
