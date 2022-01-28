use std::{
  collections::{HashMap, HashSet},
  sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
  },
  time::Instant,
};

use crossbeam::{
  channel::{self},
  queue::SegQueue,
};
use dashmap::{DashMap, DashSet};
use once_cell::sync::Lazy;
use petgraph::{
  graph::NodeIndex,
  visit::{DfsPostOrder, EdgeRef},
  EdgeDirection,
};
use rayon::prelude::*;
use swc_atoms::JsWord;
use swc_common::sync::Lrc;
use swc_common::SourceMap;

use crate::types::{IsExternal, NormalizedInputOptions};
use crate::{
  external_module::ExternalModule,
  module::Module,
  plugin_driver::PluginDriver,
  scanner::rel::{ImportInfo, ReExportInfo},
  symbol_box::SymbolBox,
  types::ResolvedId,
  utils::resolve_id,
  worker::Worker,
};

type ModuleGraph = petgraph::graph::DiGraph<String, Rel>;

pub struct GraphContainer {
  pub entries: Vec<String>,
  resolved_entries: Vec<ResolvedId>,
  pub external: Arc<Mutex<Vec<IsExternal>>>,
  pub graph: ModuleGraph,
  pub entry_indexs: Vec<NodeIndex>,
  pub ordered_modules: Vec<NodeIndex>,
  pub plugin_driver: Arc<Mutex<PluginDriver>>,
  pub symbol_box: Arc<Mutex<SymbolBox>>,
  pub modules: Arc<DashMap<String, Module>>,
  pub id_to_module: HashMap<String, Module>,
  pub resolved_ids: HashMap<(Option<JsWord>, JsWord), ResolvedId>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum ModOrExt {
  Mod(Module),
  Ext(ExternalModule),
}

// Relation between modules
#[derive(Debug)]
pub enum Rel {
  Import(ImportInfo),
  ReExport(ReExportInfo),
  ReExportAll,
}

pub enum Msg {
  DependencyReference(String, String, Rel),
  NewMod(Module),
  NewExtMod(ExternalModule),
}

impl GraphContainer {
  pub fn new(options: &NormalizedInputOptions) -> Self {
    Self {
      external: Arc::clone(&options.external),
      entries: options.input.clone(),
      resolved_entries: Default::default(),
      entry_indexs: Default::default(),
      ordered_modules: Default::default(),
      plugin_driver: Arc::new(Mutex::new(PluginDriver::from_plugins(
        options.plugins.clone(),
      ))),
      resolved_ids: Default::default(),
      id_to_module: Default::default(),
      graph: ModuleGraph::new(),
      symbol_box: Arc::new(Mutex::new(SymbolBox::new())),
      modules: Default::default(),
    }
  }

  // #[inline]
  // pub fn from_single_entry(entry: String) -> Self {
  //   Self::new(vec![entry])
  // }
  // build dependency graph via entry modules.
  fn generate_module_graph(&mut self) {
    let nums_of_thread = num_cpus::get();
    let idle_thread_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(nums_of_thread));
    let job_queue: Arc<SegQueue<ResolvedId>> = Default::default();
    self.resolved_entries = self
      .entries
      .iter()
      .map(|entry| {
        resolve_id(
          entry,
          None,
          false,
          &self.plugin_driver.lock().unwrap(),
          self.external.clone(),
        )
      })
      .collect();

    let mut path_to_node_idx: HashMap<String, NodeIndex> = Default::default();

    self.resolved_entries.iter().for_each(|resolved_entry_id| {
      let entry_idx = self.graph.add_node(resolved_entry_id.id.clone());
      self.entry_indexs.push(entry_idx);
      log::debug!("len {}", resolved_entry_id.id.bytes().len());
      path_to_node_idx.insert(resolved_entry_id.id.clone(), entry_idx);
      job_queue.push(resolved_entry_id.clone());
    });

    let processed_id: Arc<DashSet<String>> = Default::default();

    let (tx, rx) = channel::unbounded::<Msg>();

    for _ in 0..nums_of_thread {
      let idle_thread_count = idle_thread_count.clone();
      let mut worker = Worker {
        tx: tx.clone(),
        job_queue: job_queue.clone(),
        processed_id: processed_id.clone(),
        plugin_driver: self.plugin_driver.clone(),
        symbol_box: self.symbol_box.clone(),
        external: self.external.clone(),
      };
      std::thread::spawn(move || loop {
        idle_thread_count.fetch_sub(1, Ordering::SeqCst);
        worker.run();
        idle_thread_count.fetch_add(1, Ordering::SeqCst);
        loop {
          if !worker.job_queue.is_empty() {
            break;
            // need to work again
          } else if idle_thread_count.load(Ordering::SeqCst) == nums_of_thread {
            // All threads are idle now. There's no more work to do.
            return;
          }
        }
      });
    }

    while idle_thread_count.load(Ordering::SeqCst) != nums_of_thread
      || job_queue.len() > 0
      || !rx.is_empty()
    {
      if let Ok(job) = rx.try_recv() {
        match job {
          Msg::NewMod(module) => {
            self.id_to_module.insert(module.id.clone(), module);
          }
          Msg::DependencyReference(from, to, rel) => {
            let from_id = *path_to_node_idx
              .entry(from)
              .or_insert_with_key(|key| self.graph.add_node(key.clone()));
            let to_id = *path_to_node_idx
              .entry(to)
              .or_insert_with_key(|key| self.graph.add_node(key.clone()));
            self.graph.add_edge(from_id, to_id, rel);
          }
          _ => {}
        }
      }
    }

    let entries_id = self
      .entry_indexs
      .iter()
      .map(|idx| &self.graph[*idx])
      .collect::<HashSet<&String>>();
    self.id_to_module.par_iter_mut().for_each(|(_key, module)| {
      module.is_user_defined_entry_point = entries_id.contains(&module.id);
    });
  }

  fn sort_modules(&mut self) {
    let mut dfs = DfsPostOrder::new(&self.graph, self.entry_indexs[0]);
    let mut ordered_modules = vec![];
    // FIXME: The impalementation isn't correct. It’s not idempotent.
    while let Some(node) = dfs.next(&self.graph) {
      ordered_modules.push(node);
    }
    self.ordered_modules = ordered_modules;
    log::debug!("self.ordered_modules {:?}", self.ordered_modules);
  }

  pub fn build(&mut self) {
    let start = Instant::now();
    self.generate_module_graph();
    log::debug!(
      "generate_module_graph finished in {}",
      start.elapsed().as_millis()
    );

    self.sort_modules();
    log::debug!("sort_modules finished in {}", start.elapsed().as_millis());

    self.link_module_exports();
    self.link_module();
    log::debug!("link finished in {}", start.elapsed().as_millis());
    self.include_statements();
    log::debug!("build finished in {}", start.elapsed().as_millis());

    log::debug!("modules:\n{:#?}", self.id_to_module);
    log::debug!(
      "symbol_box:\n{:#?}",
      self.symbol_box.lock().unwrap().mark_uf
    );
    log::debug!(
      "grpah:\n{:?}",
      petgraph::dot::Dot::with_config(&self.graph, &[])
    );

    // log::debug!("entry_modules {:?}", Dot::new(&self.graph))
  }

  pub fn include_statements(&mut self) {
    self.id_to_module.par_iter_mut().for_each(|(_key, module)| {
      module.include();
    });
  }

  // pub fn get_module

  pub fn link_module_exports(&mut self) {
    self.ordered_modules.iter().for_each(|idx| {
      let mut dep_module_exports = vec![];

      if let Some(module) = self.id_to_module.get_mut(&self.graph[*idx]) {
        let re_export_all_ids = module
          .re_export_all_sources
          .clone()
          .iter()
          .map(|dep| module.resolve_id(dep, &self.plugin_driver, self.external.clone()))
          .collect::<Vec<_>>();

        re_export_all_ids.into_iter().for_each(|resolved_id| {
          if !resolved_id.external.unwrap_or_default() {
            let re_exported = self.id_to_module.get(&resolved_id.id).unwrap();
            re_exported.exports.clone().into_iter().for_each(|item| {
              dep_module_exports.push(item);
            });
          }
        });
      }

      if let Some(module) = self.id_to_module.get_mut(&self.graph[*idx]) {
        dep_module_exports.into_iter().for_each(|(key, mark)| {
          // TODO: we need to detect duplicate export here.
          assert!(!module.exports.contains_key(&key));
          module.exports.insert(key, mark);
        });
      }
    });
  }

  pub fn link_module(&mut self) {
    self.ordered_modules.iter().for_each(|idx| {
      let edges = self.graph.edges_directed(*idx, EdgeDirection::Outgoing);
      edges.for_each(|edge| {
        log::debug!(
          "[graph]: link module from {:?} to {:?}",
          &self.graph[*idx],
          &self.graph[edge.target()]
        );

        match edge.weight() {
          Rel::Import(info) => {
            info.names.iter().for_each(|imported| {
              if &imported.original == "default" || &imported.original == "*" {
                let module = self
                  .id_to_module
                  .get_mut(&self.graph[edge.target()])
                  .unwrap();
                module.suggest_name(imported.original.clone(), imported.used.clone());
              }

              log::debug!(
                "[graph]: link imported `{:?}` to exported {} in {}",
                imported.used,
                imported.original,
                &self
                  .id_to_module
                  .get(&self.graph[edge.target()])
                  .unwrap()
                  .id
              );

              let imported_module = self
                .id_to_module
                .get_mut(&self.graph[edge.target()])
                .unwrap();

              if &imported.original == "*" {
                imported_module.include_namespace();
              }

              let imported_module_export_mark = imported_module
                .exports
                .get(&imported.original)
                .expect("Not found");

              self
                .symbol_box
                .lock()
                .unwrap()
                .union(imported.mark, *imported_module_export_mark);

              log::debug!(
                "[graph]: link module's import {:?} to related module's mark {:?}",
                imported,
                *imported_module_export_mark
              );
            });
          }
          Rel::ReExport(info) => {
            info.names.iter().for_each(|re_exported| {
              if &re_exported.original == "default" || &re_exported.original == "*" {
                let module = self
                  .id_to_module
                  .get_mut(&self.graph[edge.target()])
                  .unwrap();
                if &re_exported.used == "default" {
                  // export { default } from './foo'
                  // Nothing we could suggest about
                } else {
                  module.suggest_name(re_exported.original.clone(), re_exported.used.clone());
                }
              }

              let re_exported_module = self
                .id_to_module
                .get_mut(&self.graph[edge.target()])
                .unwrap();

              if &re_exported.original == "*" {
                re_exported_module.include_namespace();
              }

              let re_exported_module_export_mark = self
                .id_to_module
                .get(&self.graph[edge.target()])
                .unwrap()
                .exports
                .get(&re_exported.original)
                .expect("Not found");
              self
                .symbol_box
                .lock()
                .unwrap()
                .union(re_exported.mark, *re_exported_module_export_mark);
              log::debug!(
                "[graph]: link module's re_export {:?} to related module's mark {:?}",
                re_exported,
                *re_exported_module_export_mark
              );
            });
          }
          _ => {}
        }
      });
    });
  }
}
