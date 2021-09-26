use std::collections::HashMap;
use std::io;

use once_cell::sync::Lazy;
use swc_common::DUMMY_SP;
use swc_common::{sync::Lrc, SourceMap};
use thiserror::Error;

use crate::{
  external_module::ExternalModule,
  hook_driver::HookDriver,
  module::{Module, ModuleOptions},
  types::shared::Shared,
};

pub(crate) static SOURCE_MAP: Lazy<Lrc<SourceMap>> = Lazy::new(Default::default);

#[derive(Debug, Error)]
pub enum GraphError {
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
pub struct Graph {
  pub entry: String,
  pub entry_module: Option<Shared<Module>>,
  pub modules_by_id: HashMap<String, ModOrExt>,
  pub hook_driver: HookDriver,
}

impl Graph {
  pub fn new(entry: &str) -> Graph {
    Graph {
      entry: entry.to_owned(),
      entry_module: None,
      modules_by_id: HashMap::new(),
      hook_driver: HookDriver {},
    }
  }
  // build a module using dependency relationship
  pub fn build(this: &Shared<Graph>) -> Result<swc_ecma_ast::Module, GraphError> {
    Graph::generate_module_graph(this);
    let entry_module = this.entry_module.as_ref().ok_or(GraphError::NoEntry)?;
    let statements = Module::expand_all_statements(&mut *entry_module.borrow_mut(), true);
    let body = statements.iter().map(|s| s.node.clone()).collect();

    Ok(swc_ecma_ast::Module {
      span: DUMMY_SP,
      body,
      shebang: None,
    })
  }

  // generate the entry module
  pub fn generate_module_graph(this: &Shared<Self>) {
    let nor_or_ext = Graph::fetch_module(this, &this.entry, None);
    if let Ok(ModOrExt::Mod(ref module)) = nor_or_ext {
      this.borrow_mut().entry_module.replace(module.clone());
    }
  }

  pub fn fetch_module(
    this: &Shared<Self>,
    source: &str,
    importer: Option<&str>,
  ) -> Result<ModOrExt, GraphError> {
    Ok(
      this
        .hook_driver
        .resolve_id(source, importer)
        .map(|id| {
          this.modules_by_id.get(&id).cloned().unwrap_or_else(|| {
            let source = this.hook_driver.load(&id).unwrap();
            let module = ModOrExt::Mod(Shared::new(Module::new(ModuleOptions {
              source,
              id: id.to_string(),
              graph: this.clone(),
            })));
            this
              .borrow_mut()
              .modules_by_id
              .insert(id.clone(), module.clone());
            module
          })
        })
        .unwrap_or_else(|| {
          this.modules_by_id.get(source).cloned().unwrap_or_else(|| {
            let module = ModOrExt::Ext(Shared::new(ExternalModule {
              name: source.to_owned(),
            }));
            this
              .borrow_mut()
              .modules_by_id
              .insert(source.to_owned(), module.clone());
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
