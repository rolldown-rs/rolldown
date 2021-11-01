use std::collections::{HashMap, HashSet};
use std::io;
use std::sync::Arc;

use ahash::RandomState;
use once_cell::sync::Lazy;
use swc_common::{
  sync::{Lrc, RwLock},
  SourceMap,
};
use thiserror::Error;

use crate::module::analyse::ExportDesc;
use crate::Statement;
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
  #[error("Parse module failed")]
  ParseModuleError,
}

impl From<io::Error> for GraphError {
  fn from(err: io::Error) -> Self {
    Self::IoError(err)
  }
}

#[derive(Clone)]
struct ModuleContainer {
  // cached module
  modules_by_id: RwLock<HashMap<String, ModOrExt, RandomState>>,
  internal_namespace_module_ids: HashSet<String, RandomState>,
}

impl ModuleContainer {
  pub fn new() -> Self {
    Self {
      modules_by_id: RwLock::new(HashMap::default()),
      internal_namespace_module_ids: HashSet::default(),
    }
  }

  #[inline]
  pub fn get_module(&self, id: &str) -> Option<ModOrExt> {
    self.modules_by_id.borrow().get(id).cloned()
  }

  #[inline]
  pub(crate) fn insert_module(&self, id: String, module: ModOrExt) {
    self.modules_by_id.borrow_mut().insert(id, module);
  }

  pub fn insert_internal_namespace_module_id(&mut self, id: String) {
    self.internal_namespace_module_ids.insert(id);
  }

  pub(crate) fn fetch_module(
    &self,
    source: &str,
    importer: Option<&str>,
    hook_driver: &HookDriver,
  ) -> Result<ModOrExt, GraphError> {
    hook_driver
      .resolve_id(source, importer)
      .map(|id| {
        self.get_module(&id).map(Ok).unwrap_or_else(|| {
          let source = hook_driver.load(&id).unwrap();
          if let Ok(m) = Module::new(source, id.clone()) {
            let module = ModOrExt::Mod(Arc::new(m));
            self.insert_module(id, module.clone());
            Ok(module)
          } else {
            Err(GraphError::ParseModuleError)
          }
        })
      })
      .unwrap_or_else(|| {
        self.get_module(source).map(Ok).unwrap_or_else(|| {
          let module = ModOrExt::Ext(Arc::new(ExternalModule {
            name: source.to_owned(),
          }));
          self.insert_module(source.to_owned(), module.clone());
          Ok(module)
        })
      })
  }
}

#[derive(Clone)]
#[non_exhaustive]
pub struct Graph {
  entry: String,
  entry_module: Arc<Module>,
  module_container: RwLock<ModuleContainer>,
  hook_driver: HookDriver,
}

impl Graph {
  // build a module using dependency relationship
  pub fn new(entry: &str) -> Result<Self, GraphError> {
    // generate the entry module
    let hook_driver = HookDriver::new();
    let module_container = ModuleContainer::new();
    let entry_module = module_container
      .fetch_module(entry, None, &hook_driver)?
      .into_mod()
      .expect("entry module not found");

    let graph = Self {
      entry: entry.to_owned(),
      entry_module,
      module_container: RwLock::new(module_container),
      hook_driver,
    };

    Ok(graph)
  }

  pub fn build(&self) -> Vec<Arc<Statement>> {
    log::debug!("start build for entry {:?}", self.entry);

    if let Some(ExportDesc::Default(default_export)) = self.entry_module.exports.get("default") {
      if let Some(ref name) = default_export.declared_name {
        self
          .entry_module
          .suggest_name("default".to_owned(), name.clone())
      } else {
        let default_export_name = "$$legal_identifier".to_owned();
        self
          .entry_module
          .suggest_name("default".to_owned(), default_export_name);
      }
    }

    let statements = self.entry_module.expand_all_statements(true, self);
    self.de_conflict();
    self.sort();
    statements
  }

  fn de_conflict(&self) {}

  fn sort(&self) {}

  pub fn fetch_module(&self, source: &str, importer: Option<&str>) -> Result<ModOrExt, GraphError> {
    self
      .module_container
      .borrow()
      .fetch_module(source, importer, &self.hook_driver)
  }

  pub fn get_module(&self, id: &str) -> ModOrExt {
    self.module_container.borrow().get_module(id).unwrap()
  }

  pub fn insert_internal_namespace_module_id(&self, id: String) {
    self
      .module_container
      .borrow_mut()
      .insert_internal_namespace_module_id(id);
  }
}

#[derive(Clone)]
pub enum ModOrExt {
  Mod(Arc<Module>),
  Ext(Arc<ExternalModule>),
}

impl ModOrExt {
  #[inline]
  pub fn is_mod(&self) -> bool {
    matches!(self, ModOrExt::Mod(_))
  }

  #[inline]
  pub fn is_ext(&self) -> bool {
    !self.is_mod()
  }

  pub fn into_mod(self) -> Option<Arc<Module>> {
    if let ModOrExt::Mod(m) = self {
      Some(m)
    } else {
      None
    }
  }

  pub fn into_ext(self) -> Option<Arc<ExternalModule>> {
    if let ModOrExt::Ext(m) = self {
      Some(m)
    } else {
      None
    }
  }
}
