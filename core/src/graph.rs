use std::{
  collections::{HashMap, HashSet},
  sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
  },
};

use crossbeam::{
  channel::{self},
  queue::SegQueue,
};
use dashmap::{DashMap, DashSet};
use petgraph::{graph::NodeIndex, visit::EdgeRef, EdgeDirection};
use rayon::prelude::*;
use smol_str::SmolStr;

use swc_atoms::JsWord;
use swc_common::Mark;
use swc_ecma_ast::{ModuleDecl, ModuleItem};

use crate::scanner::rel::ReExportDesc;
use crate::{
  external_module::ExternalModule,
  module::Module,
  scanner::rel::RelationInfo,
  symbol_box::SymbolBox,
  types::{NormalizedInputOptions, ResolvedId},
  utils::resolve_id,
  worker::Worker,
};

type ModulePetGraph = petgraph::graph::DiGraph<SmolStr, Rel>;

#[derive(Debug)]
pub enum MarkStmt {
  Stmt(SmolStr, usize),
  ImportNamespace(SmolStr),
  ExportNamespace(SmolStr),
}

pub struct Graph {
  pub input_options: NormalizedInputOptions,
  resolved_entries: Vec<ResolvedId>,
  pub module_graph: ModulePetGraph,
  pub entry_indexs: Vec<NodeIndex>,
  pub ordered_modules: Vec<NodeIndex>,
  pub symbol_box: Arc<Mutex<SymbolBox>>,
  pub module_by_id: HashMap<SmolStr, Box<Module>>,
  pub mark_to_stmt: Arc<DashMap<Mark, MarkStmt>>,
}

// Relation between modules
#[derive(Debug)]
pub enum Rel {
  Import(RelationInfo),
  ReExport(RelationInfo),
  ReExportAll(usize),
}

impl Rel {
  #[inline]
  fn get_order(&self) -> usize {
    match self {
      Self::Import(info) => info.order,
      Self::ReExport(info) => info.order,
      Self::ReExportAll(order) => *order,
    }
  }
}

pub enum Msg {
  DependencyReference(SmolStr, SmolStr, Rel),
  NewMod(Box<Module>),
  NewExtMod(ExternalModule),
}

impl Graph {
  pub fn new(input_options: NormalizedInputOptions) -> Self {
    Self {
      input_options,
      resolved_entries: Default::default(),
      entry_indexs: Default::default(),
      ordered_modules: Default::default(),
      module_by_id: Default::default(),
      module_graph: ModulePetGraph::new(),
      symbol_box: Arc::new(Mutex::new(SymbolBox::new())),
      mark_to_stmt: Default::default(),
    }
  }

  #[inline]
  pub fn from_single_entry(entry: String) -> Self {
    Self::new(NormalizedInputOptions {
      input: vec![entry],
      ..Default::default()
    })
  }
  // build dependency graph via entry modules.
  fn generate_module_graph(&mut self) {
    let nums_of_thread = num_cpus::get();
    let idle_thread_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(nums_of_thread));
    let job_queue: Arc<SegQueue<ResolvedId>> = Default::default();
    self.resolved_entries = self
      .input_options
      .input
      .iter()
      .map(|entry| resolve_id(entry, None, false))
      .collect();

    let mut path_to_node_idx: HashMap<SmolStr, NodeIndex> = Default::default();

    self.resolved_entries.iter().for_each(|resolved_entry_id| {
      let entry_idx = self.module_graph.add_node(resolved_entry_id.id.clone());
      self.entry_indexs.push(entry_idx);
      path_to_node_idx.insert(resolved_entry_id.id.clone(), entry_idx);
      job_queue.push(resolved_entry_id.clone());
    });

    let processed_id: Arc<DashSet<SmolStr>> = Default::default();

    let (tx, rx) = channel::unbounded::<Msg>();

    for _ in 0..nums_of_thread {
      let idle_thread_count = idle_thread_count.clone();
      let mut worker = Worker {
        tx: tx.clone(),
        job_queue: job_queue.clone(),
        processed_id: processed_id.clone(),
        symbol_box: self.symbol_box.clone(),
        mark_to_stmt: self.mark_to_stmt.clone(),
      };
      std::thread::spawn(move || loop {
        idle_thread_count.fetch_sub(1, Ordering::SeqCst);
        if let Err(e) = worker.run() {
          eprintln!("{}", e);
        }
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
            self.module_by_id.insert(module.id.clone(), module);
          }
          Msg::DependencyReference(from, to, rel) => {
            let from_id = *path_to_node_idx
              .entry(from)
              .or_insert_with_key(|key| self.module_graph.add_node(key.clone()));
            let to_id = *path_to_node_idx
              .entry(to)
              .or_insert_with_key(|key| self.module_graph.add_node(key.clone()));
            self.module_graph.add_edge(from_id, to_id, rel);
          }
          _ => {}
        }
      }
    }

    let entries_id = self
      .entry_indexs
      .iter()
      .map(|idx| &self.module_graph[*idx])
      .collect::<HashSet<&SmolStr>>();
    self.module_by_id.par_iter_mut().for_each(|(_key, module)| {
      module.is_user_defined_entry_point = entries_id.contains(&module.id);
    });
  }

  fn sort_modules(&mut self) {
    let mut ordered_modules = vec![];
    let entry = self.entry_indexs[0];
    let mut visited = HashSet::new();
    let mut stack = vec![entry];
    while let Some(node_idx) = stack.pop() {
      if !visited.contains(&node_idx) {
        stack.push(node_idx);
        visited.insert(node_idx);
        let edges = self
          .module_graph
          .edges_directed(node_idx, EdgeDirection::Outgoing);
        let mut rels = edges.collect::<Vec<_>>();
        rels.sort_by_key(|a| a.weight().get_order());
        rels
          .into_iter()
          .rev()
          .filter(|edge| !visited.contains(&edge.target()))
          .for_each(|edge| stack.push(edge.target()));
      } else {
        ordered_modules.push(node_idx);
      }
      // ordered_modules = ordered_modules.into_iter().rev().collect()
    }

    // while let Some(node) = dfs.next(&self.graph) {
    //   ordered_modules.push(node);
    // }
    self.ordered_modules = ordered_modules;
    // println!("self.ordered_modules {:#?}", ordered_modules.iter().map(|idx| &self.graph[*idx]).collect::<Vec<_>>());
  }

  pub fn build(&mut self) {
    self.generate_module_graph();
    self.sort_modules();
    self.link_module_exports();
    self.link_module();
    self.include();

    println!(
      "symbol_box after build {:#?}",
      self.symbol_box.lock().unwrap()
    );
  }

  pub fn include(&mut self) {
    let treeshake = self.input_options.treeshake;
    self.module_by_id.par_iter_mut().for_each(|(id, module)| {
      log::debug!(
        "[treeshake]: with treeshake: {:?}, include all module's side effect stmt for {:?}",
        treeshake,
        id
      );
      module.include(treeshake);
    });

    if treeshake {
      self.resolved_entries.iter().for_each(|resolved_id| {
        log::debug!(
          "[treeshake]: include entry module's local exports for {:?}",
          resolved_id.id
        );

        // include top-level exports namespace
        let module = self.module_by_id.get(&resolved_id.id.clone()).unwrap();

        let mut export_namespaced: Option<(JsWord, ReExportDesc)> = None;
        module.re_exports.iter().for_each(|(name, desc)| {
          if &desc.local_name == "*" {
            export_namespaced = Some((name.clone(), desc.clone()))
          }
        });

        if let Some((_, desc)) = export_namespaced {
          let dep_module_id = module.resolved_ids.get(&desc.source).unwrap().clone().id;
          let dep_module = self.module_by_id.get_mut(dep_module_id.as_str()).unwrap();
          dep_module.include_namespace();
        }

        // include other top-level exports
        let module = self.module_by_id.get_mut(&resolved_id.id.clone()).unwrap();

        module
          .local_exports
          .values()
          .map(|desc| (desc.local_name.clone(), desc.mark))
          .collect::<Vec<_>>()
          .into_iter()
          .for_each(|(name, mark)| {
            module.include_mark(&name, &mark);
          });
      });

      let dummy_marks: HashSet<Mark> = Default::default();
      let mut visited_marks: HashSet<Mark> = Default::default();

      #[allow(clippy::needless_collect)]
      let mut read_marks = self
        .module_by_id
        .iter()
        .flat_map(|(_id, module)| {
          module.statements.iter().flat_map(|stmt| {
            // export { a }, export * as foo from "./foo", export default a, is not regarded as reads, the real read place is where we consume(read or entry-point exports) those variables, so we have to exclude it
            // top-level exports is already handled above
            if !matches!(
              stmt.node,
              ModuleItem::ModuleDecl(|ModuleDecl::ExportDefaultExpr(_)| ModuleDecl::ExportNamed(_))
            ) {
              stmt.reads.iter()
            } else {
              dummy_marks.iter()
            }
          })
        })
        .cloned()
        .collect::<Vec<_>>();

      while let Some(mark) = read_marks.pop() {
        if visited_marks.contains(&mark) {
          continue;
        }
        visited_marks.insert(mark.clone());

        let from_root_mark = self.symbol_box.lock().unwrap().find_root(mark);
        let matched_decls = self.mark_to_stmt.iter().filter(|pair| {
          let dest_root_mark = self.symbol_box.lock().unwrap().find_root(*pair.key());
          from_root_mark == dest_root_mark
        });

        matched_decls
          .into_iter()
          .for_each(|pair| match pair.value() {
            MarkStmt::Stmt(module_id, idx) => {
              let module = self.module_by_id.get_mut(module_id).unwrap();
              let stmt = &mut module.statements[*idx];
              if !stmt.included {
                log::debug!("[treeshake]: include statement {:?}", stmt.node.clone());
                stmt.include();
              }
            }
            MarkStmt::ImportNamespace(module_id) => {
              let module = self.module_by_id.get_mut(module_id).unwrap();
              log::debug!(
                "[treeshake]: import namespace visited, including it... {}",
                module_id
              );
              let discovered_marks = module.include_namespace();
              read_marks.extend(discovered_marks);
            }
            MarkStmt::ExportNamespace(module_id) => {
              let module = self.module_by_id.get_mut(module_id).unwrap();
              log::debug!(
                "[treeshake]: export namespace visited, including it... {}",
                module_id
              );
              let discovered_marks = module.include_namespace();
              read_marks.extend(discovered_marks);
            }
          });
      }
    }
  }

  pub fn link_module_exports(&mut self) {
    self.ordered_modules.iter().for_each(|idx| {
      let module_id = &self.module_graph[*idx];
      let module = self.module_by_id.get(module_id).unwrap();
      // self.module_by_id.get_mut
      let dep_ids = module
        .re_export_all_sources
        .iter()
        .map(|dep_src| module.resolved_ids.get(dep_src).unwrap().clone().id)
        .collect::<Vec<_>>();
      let dep_exports = dep_ids
        .into_par_iter()
        .map(|id| self.module_by_id.get(&id).unwrap())
        .map(|dep_module| (dep_module.id.clone(), dep_module.exports.clone()))
        .collect::<Vec<_>>();

      let module = self.module_by_id.get_mut(module_id).unwrap();
      dep_exports.into_iter().for_each(|(dep_id, dep_exports)| {
        dep_exports.into_iter().for_each(|(exported_name, mark)| {
          assert!(
            !module.exports.contains_key(&exported_name),
            "duplicate when export {:?} from {:?} in {:?}",
            exported_name,
            dep_id,
            module.id
          );
          module.exports.insert(exported_name, mark);
        });
      });
    });
  }

  pub fn link_module(&mut self) {
    self.ordered_modules.iter().for_each(|idx| {
      let edges = self
        .module_graph
        .edges_directed(*idx, EdgeDirection::Outgoing);
      edges.for_each(|edge| {
        log::debug!(
          "[graph]: link module from {:?} to {:?}",
          &self.module_graph[*idx],
          &self.module_graph[edge.target()]
        );
        let rel_info = match edge.weight() {
          Rel::Import(info) => Some(info),
          Rel::ReExport(info) => Some(info),
          _ => None,
        };
        if let Some(rel_info) = rel_info {
          rel_info.names.iter().for_each(|specifier| {
            let dep_module = self
              .module_by_id
              .get_mut(&self.module_graph[edge.target()])
              .unwrap();
            // import _default from './foo'
            // import * as foo from './foo
            // export * as foo from './foo
            if &specifier.original == "default" || &specifier.original == "*" {
              // There is only one case where `specifier.used` is not a valid varible name.
              // Special case ` export { default } from ...`
              if &specifier.used != "default" {
                dep_module.suggest_name(specifier.original.clone(), specifier.used.clone());
              }
            }

            log::debug!(
              "[graph]: link imported `{:?}` to exported {} in {}",
              specifier.used,
              specifier.original,
              dep_module.id
            );

            if &specifier.original == "*" {
              // link `import * as foo` to dep module's namespace mark
              println!(
                "link module {:#?} {:#?} depmoduleid {}",
                specifier, dep_module.namespace, dep_module.id
              );
              self
                .symbol_box
                .lock()
                .unwrap()
                .union(specifier.mark, dep_module.namespace.mark);
              // NOTE: we include namespace in `include` stage
              // dep_module.include_namespace(self.mark_to_stmt.clone());
            } else if let Some(dep_module_exported_mark) =
              dep_module.exports.get(&specifier.original)
            {
              self
                .symbol_box
                .lock()
                .unwrap()
                .union(specifier.mark, *dep_module_exported_mark);
            }
          });
        }
      });
    });
  }
}
