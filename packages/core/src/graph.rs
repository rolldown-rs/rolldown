use crate::{
  external_module::ExternalModule,
  hook_driver::HookDriver,
  module::{Module, ModuleOptions},
  statement::Statement,
  types::shared::Shared,
};
use path_absolutize::Absolutize;
use std::{
  borrow::BorrowMut,
  cell::RefCell,
  collections::HashMap,
  env, fs,
  io::stdout,
  path::{Path, PathBuf},
  rc::Rc,
  vec,
};
use swc_common::{
  errors::{ColorConfig, Handler},
  sync::Lrc,
  FileName, FilePathMapping, SourceMap,
};
use swc_common::{BytePos, Span, SyntaxContext, DUMMY_SP};
use swc_ecma_codegen::{text_writer::JsWriter, Emitter};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};

fn resolve(path: &str) -> PathBuf {
  Path::join(env::current_dir().unwrap().as_path(), path)
}

#[derive(Clone)]
pub struct Graph {
  pub cm: Lrc<SourceMap>,
  pub entry: String,
  pub entry_module: Option<Shared<Module>>,
  pub modules_by_id: HashMap<String, ModOrExt>,
  pub hook_driver: HookDriver,
  // pub external_modules: Vec<ExternalModule>,
  // pub internal_namespace_modules: Vec<String>,
}

impl Graph {
  pub fn new(entry: &str) -> Graph {
    let cm: Lrc<SourceMap> = Default::default();
    Graph {
      cm,
      entry: entry.to_owned(),
      entry_module: None,
      modules_by_id: HashMap::new(),
      hook_driver: HookDriver {},
      // var_exports: HashMap::new(),
      // to_export: None,
      // statements: vec![],
      // external_modules: vec![],
      // internal_namespace_modules: vec![],
      // assumed_globals: HashMap::new(),
    }
  }
  // build a module using dependcy relationship
  pub fn build(this: &Shared<Graph>) -> swc_ecma_ast::Module {
    Graph::generate_module_graph(this);
    let entry_module = this.entry_module.as_ref().unwrap().clone();
    let statements = Module::expand_all_statements(&mut *entry_module.borrow_mut(), true);
    let body = statements.iter().map(|s| s.node.clone()).collect();
    let result = swc_ecma_ast::Module {
      span: DUMMY_SP,
      body,
      shebang: None,
    };
    result
  }

  // generate the entry module
  pub fn generate_module_graph(this: &Shared<Self>) {
    let nor_or_ext = Graph::fetch_module(this, &this.entry, None);
    if let ModOrExt::Mod(module) = &nor_or_ext {
      this.borrow_mut().entry_module.replace(module.clone());
    }
  }

  pub fn fetch_module(this: &Shared<Self>, source: &str, importer: Option<&str>) -> ModOrExt {
    let maybe_id = this.hook_driver.resolve_id(source, importer);
    if let Some(id) = &maybe_id {
      if let Some(module) = this.modules_by_id.get(id) {
        module.clone()
      } else {
        let source = this.hook_driver.load(id);
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
      }
    } else {
      if let Some(module) = this.modules_by_id.get(source) {
        module.clone()
      } else {
        let module = ModOrExt::Ext(Shared::new(ExternalModule {
          name: source.to_owned(),
        }));
        this
          .borrow_mut()
          .modules_by_id
          .insert(source.to_owned(), module.clone());
        module
      }
    }
  }
}

#[derive(Clone)]
pub enum ModOrExt {
  Mod(Shared<Module>),
  Ext(Shared<ExternalModule>),
}

impl ModOrExt {
  pub fn is_mod(&self) -> bool {
    if let ModOrExt::Mod(_) = self {
      true
    } else {
      false
    }
  }
  pub fn is_ext(&self) -> bool {
    !self.is_mod()
  }
}
