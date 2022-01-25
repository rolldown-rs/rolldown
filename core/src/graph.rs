use std::{
  collections::HashMap,
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
  pub entry_path: String,
  pub graph: ModuleGraph,
  pub entries: Vec<NodeIndex>,
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

pub(crate) static SOURCE_MAP: Lazy<Lrc<SourceMap>> = Lazy::new(Default::default);

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
  pub fn new(entry_path: String) -> Self {
    Self {
      entry_path,
      entries: Default::default(),
      ordered_modules: Default::default(),
      plugin_driver: Arc::new(Mutex::new(PluginDriver::new())),
      resolved_ids: Default::default(),
      id_to_module: Default::default(),
      graph: ModuleGraph::new(),
      symbol_box: Arc::new(Mutex::new(SymbolBox::new())),
      modules: Default::default(),
    }
  }

  // build dependency graph via entry modules.
  fn generate_module_graph(&mut self) {
    let nums_of_thread = num_cpus::get_physical();
    let idle_thread_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(nums_of_thread));
    let job_queue: Arc<SegQueue<ResolvedId>> = Default::default();
    let entry_id = resolve_id(
      &self.entry_path,
      None,
      false,
      &self.plugin_driver.lock().unwrap(),
    );
    let mut path_to_node_idx: HashMap<String, NodeIndex> = Default::default();

    let entry_idx = self.graph.add_node(entry_id.id.clone());
    self.entries.push(entry_idx);
    path_to_node_idx.insert(entry_id.id.clone(), entry_idx);
    println!("entry_id {:?}", entry_id);
    job_queue.push(entry_id);

    // let id_to_module: Arc<DashMap<String, Module>> = self.id_to_module.clone();
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
            log::debug!("end thread");
            return;
          }
        }
      });
    }

    while idle_thread_count.load(Ordering::SeqCst) != nums_of_thread
      || job_queue.len() > 0
      || !rx.is_empty()
    {
      // println!("active_count {}", active_count.load(Ordering::SeqCst));
      if let Ok(job) = rx.try_recv() {
        match job {
          Msg::NewMod(module) => {
            self.id_to_module.insert(module.id.clone(), module);
          }
          Msg::DependencyReference(from, to, rel) => {
            let from_id = *path_to_node_idx
              .entry(from.clone())
              .or_insert_with(|| self.graph.add_node(from));
            let to_id = *path_to_node_idx
              .entry(to.clone())
              .or_insert_with(|| self.graph.add_node(to));
            self.graph.add_edge(from_id, to_id, rel);
          }
          _ => {}
        }
      }
    }
  }

  fn sort_modules(&mut self) {
    let mut dfs = DfsPostOrder::new(&self.graph, self.entries[0]);
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
    println!(
      "generate_module_graph finished in {}",
      start.elapsed().as_millis()
    );

    self.sort_modules();
    println!("sort_modules finished in {}", start.elapsed().as_millis());

    self.link_module_exports();
    self.link_module();
    println!("link finished in {}", start.elapsed().as_millis());
    self.include_statements();
    println!("build finished in {}", start.elapsed().as_millis());

    log::debug!("id_to_module {:#?}", self.id_to_module);
    log::debug!("symbol_box {:#?}", self.symbol_box.lock());
    log::debug!(
      "grpah {:?}",
      petgraph::dot::Dot::with_config(&self.graph, &[])
    );

    // println!("entry_modules {:?}", Dot::new(&self.graph))
  }

  pub fn include_statements(&mut self) {
    self.id_to_module.par_iter_mut().for_each(|(_key, module)| {
      module.include();
    });
  }

  pub fn link_module_exports(&mut self) {
    self.ordered_modules.iter().for_each(|idx| {
      let mut dep_module_exports = vec![];

      if let Some(module) = self.id_to_module.get_mut(&self.graph[*idx]) {
        let re_export_all_ids = module
          .re_export_all_sources
          .clone()
          .iter()
          .map(|dep| module.resolve_id(dep, &self.plugin_driver))
          .collect::<Vec<_>>();

        re_export_all_ids.into_iter().for_each(|resolved_id| {
          if !resolved_id.external {
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
          module.exports.insert(key, mark);
        });
      }
    });
  }

  pub fn link_module(&mut self) {
    self.ordered_modules.iter().for_each(|idx| {
      let edges = self.graph.edges_directed(*idx, EdgeDirection::Outgoing);
      edges.for_each(|edge| {
        // let imported_or_re_exported_module =

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

              let imported_module_export_mark = self
                .id_to_module
                .get(&self.graph[edge.target()])
                .unwrap()
                .exports
                .get(&imported.original)
                .expect("Not found");
              self
                .symbol_box
                .lock()
                .unwrap()
                .union(imported.mark, *imported_module_export_mark);
            });
          }
          Rel::ReExport(info) => {
            info.names.iter().for_each(|re_exported| {
              if &re_exported.original == "default" || &re_exported.original == "*" {
                let module = self
                  .id_to_module
                  .get_mut(&self.graph[edge.target()])
                  .unwrap();
                module.suggest_name(re_exported.original.clone(), re_exported.used.clone());
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
            });
          }
          _ => {}
        }
      });
    });
  }
}
