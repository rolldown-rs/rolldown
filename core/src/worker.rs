use std::{
  fs,
  sync::{Arc, Mutex},
};

use crossbeam::{channel::Sender, queue::SegQueue};
use dashmap::{DashMap, DashSet};
use smol_str::SmolStr;
use swc_common::Mark;
use swc_ecma_ast::{ModuleDecl, ModuleItem};
use swc_ecma_visit::VisitMutWith;
use thiserror::Error;

use crate::{
  graph::{Msg, Rel},
  module::Module,
  scanner::{scope::BindType, Scanner},
  symbol_box::SymbolBox,
  types::ResolvedId,
  utils::parse_file,
};

#[derive(Error, Debug)]
pub enum RolldownError {
  #[error("[IO error `{0}`]")]
  IO(std::io::Error),
  #[error("[Crossbeam error `{0}`]")]
  Channel(crossbeam::channel::SendError<Msg>),
  #[error("[Mutex error]")]
  Lock,
}

pub struct Worker {
  pub symbol_box: Arc<Mutex<SymbolBox>>,
  pub job_queue: Arc<SegQueue<ResolvedId>>,
  pub tx: Sender<Msg>,
  pub processed_id: Arc<DashSet<SmolStr>>,
  pub mark_to_stmt: Arc<DashMap<Mark, (SmolStr, usize)>>,
}

impl Worker {
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

  pub fn run(&mut self) -> Result<(), RolldownError> {
    if let Some(resolved_id) = self.fetch_job() {
      if resolved_id.external {
        // TODO: external module
      } else {
        let mut module = Module::new(resolved_id.id.clone());
        let id: &str = &resolved_id.id;
        let source = fs::read_to_string(id).map_err(RolldownError::IO)?;
        let mut ast = parse_file(source, &module.id);
        self.pre_analyze_imported_module(&mut module, &ast);

        let mut scanner = Scanner::new(self.symbol_box.clone(), self.tx.clone());
        ast.visit_mut_with(&mut scanner);

        scanner
          .import_infos
          .iter()
          .try_for_each(|(imported, info)| {
            let resolved_id = module.resolve_id(imported);
            self
              .tx
              .send(Msg::DependencyReference(
                module.id.clone(),
                resolved_id.id,
                info.clone().into(),
              ))
              .map_err(RolldownError::Channel)
          })?;
        scanner
          .re_export_infos
          .iter()
          .try_for_each(|(re_exported, info)| {
            let resolved_id = module.resolve_id(re_exported);
            self
              .tx
              .send(Msg::DependencyReference(
                module.id.clone(),
                resolved_id.id,
                info.clone().into(),
              ))
              .map_err(RolldownError::Channel)
          })?;
        scanner
          .export_all_sources
          .iter()
          .try_for_each(|re_exported| {
            let resolved_id = module.resolve_id(&re_exported.0);
            self
              .tx
              .send(Msg::DependencyReference(
                module.id.clone(),
                resolved_id.id,
                Rel::ReExportAll(re_exported.1),
              ))
              .map_err(RolldownError::Channel)
          })?;

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
        module.namespace.mark = self
          .symbol_box
          .lock()
          .map_err(|_| RolldownError::Lock)?
          .new_mark();

        module.set_statements(ast, scanner.statement_infos, self.mark_to_stmt.clone());

        module.bind_local_references(&mut self.symbol_box.lock().unwrap());

        module.link_local_exports();

        log::debug!("[worker]: emit module {:#?}", module);
        self
          .tx
          .send(Msg::NewMod(Box::new(module)))
          .map_err(RolldownError::Channel)?;
      }
    }
    Ok(())
  }

  // Fast path for analyzing static import and export.
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
