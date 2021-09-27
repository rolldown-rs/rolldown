use std::collections::HashMap;
use std::io;

use once_cell::sync::Lazy;
use swc_common::sync::RwLock;
use swc_common::DUMMY_SP;
use swc_common::{sync::Lrc, SourceMap};
use thiserror::Error;

use crate::{
  external_module::ExternalModule, hook_driver::HookDriver, module::Module, types::shared::Shared,
};

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
  pub entry_module: Option<Shared<Module>>,
  pub modules_by_id: RwLock<HashMap<String, ModOrExt>>,
  pub hook_driver: HookDriver,
}

impl Graph {
  // build a module using dependency relationship
  pub fn build(entry: &str) -> Result<Shared<Self>, GraphError> {
    // generate the entry module
    let hook_driver = HookDriver::new();
    let modules_by_id = RwLock::new(HashMap::new());
    let mut real_modules_by_id = HashMap::new();
    let id = hook_driver
      .resolve_id(entry, None)
      .ok_or_else(|| GraphError::EntryNotFound(entry.to_owned()))?;
    let source = hook_driver.load(&id)?;
    let ret = Shared::new(Self {
      entry: entry.to_owned(),
      entry_module: None,
      modules_by_id,
      hook_driver,
    });
    let entry_module = Shared::new(Module::new(source, id.to_string(), ret.clone()));
    let module = ModOrExt::Mod(entry_module.clone());
    real_modules_by_id.insert(id.clone(), module.clone());
    ret.borrow_mut().entry_module = Some(entry_module);
    ret.borrow_mut().modules_by_id = RwLock::new(real_modules_by_id);
    Ok(ret)
  }

  pub fn get_swc_module(&self) -> Option<swc_ecma_ast::Module> {
    let statements = Module::expand_all_statements(self.entry_module.as_ref()?, true);
    let body = statements.iter().map(|s| s.node.clone()).collect();

    Some(swc_ecma_ast::Module {
      span: DUMMY_SP,
      body,
      shebang: None,
    })
  }

  pub(crate) fn get_module(&self, id: &str) -> Option<ModOrExt> {
    let read_guard = self.modules_by_id.read();
    read_guard.get(id).cloned()
  }

  pub(crate) fn insert_module(&self, id: String, module: ModOrExt) {
    let mut write_guard = self.modules_by_id.write();
    write_guard.insert(id, module);
  }

  pub(crate) fn fetch_module(
    this: &Shared<Self>,
    source: &str,
    importer: Option<&str>,
  ) -> Result<ModOrExt, GraphError> {
    Ok(
      this
        .hook_driver
        .resolve_id(source, importer)
        .map(|id| {
          this.get_module(&id).unwrap_or_else(|| {
            let source = this.hook_driver.load(&id).unwrap();
            let module = ModOrExt::Mod(Shared::new(Module::new(
              source,
              id.to_string(),
              this.clone(),
            )));
            this.insert_module(id.clone(), module.clone());
            module
          })
        })
        .unwrap_or_else(|| {
          this.get_module(source).unwrap_or_else(|| {
            let module = ModOrExt::Ext(Shared::new(ExternalModule {
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
  Mod(Shared<Module>),
  Ext(Shared<ExternalModule>),
}

impl ModOrExt {
  pub fn is_mod(&self) -> bool {
    matches!(self, ModOrExt::Mod(_))
  }
  pub fn is_ext(&self) -> bool {
    !self.is_mod()
  }
}
