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
use crate::module_loader::ModuleLoader;
use crate::types::{shared, Shared};
use crate::Statement;
use crate::utils::plugin_driver::PluginDriver;
use crate::{external_module::ExternalModule, module::Module};

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
#[non_exhaustive]
pub struct Graph {
  entry: String,
  entry_modules: Vec<Shared<Module>>,
  module_container: Shared<ModuleLoader>,
  plugin_driver: Shared<PluginDriver>,
}

impl Graph {
  // build a module using dependency relationship
  pub fn new(entry: &str, plugin_driver: Shared<PluginDriver>) -> Result<Self, GraphError> {
    // generate the entry module
    let module_container = ModuleLoader::new(entry.to_owned(), plugin_driver.clone());

    let graph = Self {
      entry: entry.to_owned(),
      entry_modules: vec![],
      module_container,
      plugin_driver,
    };

    Ok(graph)
  }

  fn generate_module_graph(&self) {
    let entry_module = self.module_container.borrow_mut().add_entry_modules(&self.entry, true);

  }

  fn build(&self) {
    self.generate_module_graph();

    self.plugin_driver.borrow().build_end()
  }

  // pub fn build(&self) -> Vec<Arc<Statement>> {
  //   log::debug!("start build for entry {:?}", self.entry);

  //   if let Some(ExportDesc::Default(default_export)) = self.entry_module.exports.get("default") {
  //     if let Some(ref name) = default_export.declared_name {
  //       self
  //         .entry_module
  //         .suggest_name("default".to_owned(), name.clone())
  //     } else {
  //       let default_export_name = "$$legal_identifier".to_owned();
  //       self
  //         .entry_module
  //         .suggest_name("default".to_owned(), default_export_name);
  //     }
  //   }

  //   let statements = self.entry_module.expand_all_statements(true, self);
  //   self.de_conflict();
  //   self.sort();
  //   statements
  // }

  // fn de_conflict(&self) {}

  // fn sort(&self) {}

  // pub fn fetch_module(&self, source: &str, importer: Option<&str>) -> Result<ModOrExt, GraphError> {
  //   self
  //     .module_container
  //     .borrow()
  //     .fetch_module(source, importer, &self.hook_driver)
  // }

  // pub fn get_module(&self, id: &str) -> ModOrExt {
  //   self.module_container.borrow().get_module(id).unwrap()
  // }

  // pub fn insert_internal_namespace_module_id(&self, id: String) {
  //   self
  //     .module_container
  //     .borrow_mut()
  //     .insert_internal_namespace_module_id(id);
  // }
}

#[derive(Clone)]
pub enum ModOrExt {
  Mod(Shared<Module>),
  Ext(Shared<ExternalModule>),
}

impl std::convert::From<Shared<ExternalModule>> for ModOrExt {
  fn from(ext: Shared<ExternalModule>) -> Self {
    ModOrExt::Ext(ext)
  }
}

impl std::convert::From<Shared<Module>> for ModOrExt {
  fn from(m: Shared<Module>) -> Self {
    ModOrExt::Mod(m)
  }
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

  pub fn into_mod(self) -> Option<Shared<Module>> {
    if let ModOrExt::Mod(m) = self {
      Some(m)
    } else {
      None
    }
  }

  pub fn into_ext(self) -> Option<Shared<ExternalModule>> {
    if let ModOrExt::Ext(m) = self {
      Some(m)
    } else {
      None
    }
  }
}
