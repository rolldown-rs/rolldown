use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::time;

use ahash::RandomState;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use swc_common::{
  sync::{Lrc, RwLock},
  SourceMap,
};
use thiserror::Error;

use crate::{external_module::ExternalModule, hook_driver::HookDriver, module::Module};

pub(crate) static SOURCE_MAP: Lazy<Lrc<SourceMap>> = Lazy::new(Default::default);

#[derive(Debug, Error)]
pub enum GraphError {
  #[error("Entry [{0}] not found")]
  EntryNotFound(String),
  #[error("Bundle doesn't have any entry")]
  NoEntry,
  #[error("{0}")]
  IoError(io::Error),
}

impl From<io::Error> for GraphError {
  fn from(err: io::Error) -> Self {
    Self::IoError(err)
  }
}

#[derive(Clone)]
#[non_exhaustive]
pub struct Graph {
  pub entry: String,
  pub entry_module: Arc<Module>,
  pub modules_by_id: Arc<RwLock<HashMap<String, ModOrExt, RandomState>>>,
  pub hook_driver: HookDriver,
  pub(crate) parent_dir_cache: RwLock<HashMap<String, String, RandomState>>,
}

impl Graph {
  // build a module using dependency relationship
  pub fn build(entry: &str) -> Result<Arc<Self>, GraphError> {
    // generate the entry module
    let hook_driver = HookDriver::new();
    let modules_by_id: Arc<RwLock<HashMap<String, ModOrExt, RandomState>>> =
      Arc::new(RwLock::new(HashMap::default()));
    let mut real_modules_by_id: HashMap<String, ModOrExt, RandomState> = HashMap::default();
    let parent_dir_cache = RwLock::new(HashMap::default());
    let id = hook_driver
      .resolve_id(entry, None, &parent_dir_cache)
      .ok_or_else(|| GraphError::EntryNotFound(entry.to_owned()))?;
    let source = hook_driver.load(&id)?;
    let mut ret = Arc::new(Self {
      entry: entry.to_owned(),
      entry_module: Arc::new(Module::empty()),
      modules_by_id,
      hook_driver,
      parent_dir_cache,
    });
    let entry_module = Arc::new(Module::new(source, id.to_string(), &ret));
    let graph = Arc::make_mut(&mut ret);
    real_modules_by_id.insert(id, ModOrExt::Mod(entry_module.clone()));
    graph.entry_module = entry_module;
    graph.modules_by_id = Arc::new(RwLock::new(real_modules_by_id));
    Ok(ret)
  }

  pub fn get_swc_module_items<F>(&self, codegen: F)
  where
    F: FnOnce(Vec<swc_ecma_ast::ModuleItem>),
  {
    let collect_all_modules_duration = time::Instant::now();
    let modules = Module::expand_all_modules(&self.entry_module, true);

    codegen(modules);
  }

  pub(crate) fn get_module<'a>(&'a self, id: &str) -> Option<ModOrExt> {
    let read_guard = self.modules_by_id.read();
    read_guard.get(id).cloned()
  }

  pub(crate) fn insert_module(&self, id: String, module: ModOrExt) {
    let mut write_guard = self.modules_by_id.write();
    write_guard.insert(id, module);
  }

  pub(crate) fn fetch_module(
    this: &Arc<Self>,
    source: &str,
    importer: Option<&str>,
  ) -> Result<ModOrExt, GraphError> {
    Ok(
      this
        .hook_driver
        .resolve_id(source, importer, &this.parent_dir_cache)
        .map(|id| {
          this.get_module(&id).unwrap_or_else(|| {
            let source = this.hook_driver.load(&id).unwrap();
            let module = ModOrExt::Mod(Arc::new(Module::new(source, id.to_string(), this)));
            this.insert_module(id.clone(), module.clone());
            module
          })
        })
        .unwrap_or_else(|| {
          this.get_module(source).unwrap_or_else(|| {
            let module = ModOrExt::Ext(Arc::new(ExternalModule {
              name: source.to_owned(),
            }));
            this.insert_module(source.to_owned(), module.clone());
            module
          })
        }),
    )
  }
}

#[derive(Clone)]
pub enum ModOrExt {
  Mod(Arc<Module>),
  Ext(Arc<ExternalModule>),
}

impl ModOrExt {
  pub fn is_mod(&self) -> bool {
    matches!(self, ModOrExt::Mod(_))
  }
  pub fn is_ext(&self) -> bool {
    !self.is_mod()
  }
}
