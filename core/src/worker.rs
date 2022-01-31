use std::sync::{Arc, Mutex};

use crossbeam::{channel::Sender, queue::SegQueue};
use dashmap::DashSet;
use rayon::prelude::*;

use smol_str::SmolStr;
use swc_ecma_ast::{ModuleDecl, ModuleItem};
use swc_ecma_visit::VisitMutWith;

use crate::{
  graph::{Msg, Rel},
  module::Module,
  scanner::{scope::BindType, Scanner},
  symbol_box::SymbolBox,
  types::ResolvedId,
  utils::{load, parse_file},
};

pub struct Worker {
  pub symbol_box: Arc<Mutex<SymbolBox>>,
  pub job_queue: Arc<SegQueue<ResolvedId>>,
  pub tx: Sender<Msg>,
  pub processed_id: Arc<DashSet<SmolStr>>,
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
        let source = load(&resolved_id.id);
        let mut ast = parse_file(source, &module.id);
        self.pre_analyze_imported_module(&mut module, &ast);

        let mut scanner = Scanner::new(self.symbol_box.clone(), self.tx.clone());
        ast.visit_mut_with(&mut scanner);

        scanner.import_infos.iter().for_each(|(imported, info)| {
          let resolved_id = module.resolve_id(imported);
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
            let resolved_id = module.resolve_id(re_exported);
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
          let resolved_id = module.resolve_id(&re_exported.0);
          self
            .tx
            .send(Msg::DependencyReference(
              module.id.clone(),
              resolved_id.id,
              Rel::ReExportAll(re_exported.1),
            ))
            .unwrap();
        });

        module.local_exports = scanner.local_exports;
        module.re_exports = scanner.re_exports;
        module.re_export_all_sources = scanner
          .export_all_sources
          .into_iter()
          .map(|s| s.0)
          .collect();
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

        module.set_ast(ast.clone(), scanner.statement_infos);

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
          let resolved_id = module.resolve_id(depended);
          self.job_queue.push(resolved_id);
        }
      }
    });
  }
}
