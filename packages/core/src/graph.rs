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

struct InputOptions {
  entry: String,
}

#[derive(Clone)]
pub struct Graph {
  pub cm: Lrc<SourceMap>,
  pub entry: String,
  pub entry_module: Option<Shared<Module>>,
  pub modules: HashMap<String, NorOrExt>,
  pub hook_driver: HookDriver,
  // pub statements: Vec<Statement<'a>>,
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
      modules: HashMap::new(),
      hook_driver: HookDriver {},
      // var_exports: HashMap::new(),
      // to_export: None,
      // statements: vec![],
      // external_modules: vec![],
      // internal_namespace_modules: vec![],
      // assumed_globals: HashMap::new(),
    }
  }

  pub fn build(this: &Shared<Graph>) {
    Graph::generate_module_graph(this);
    let shared_entry_module = Graph::fetch_module(this, &this.entry, None);
    let entry_module = this.entry_module.as_ref().unwrap().clone();
    // this.borrow_mut().entry_module.replace(entry_module.clone());
    if let Some(default_export) = entry_module.exports.get("default") {
      todo!("defaultExport")
    };
    let statements = Module::expand_all_statements(&mut *entry_module.borrow_mut(), true);
    let body = statements
      .iter()
      .map(|s| {
        println!("statements {:?}", s.node);
        s.node.clone()
      })
      .collect();
    let result = swc_ecma_ast::Module {
      span: DUMMY_SP,
      body,
      shebang: None,
    };
    println!("bundle output:");
    this.to_source(&result);
  }

  fn to_source(&self, node: &swc_ecma_ast::Module) {
    let wr = stdout();
    let mut emitter = Emitter {
      cfg: swc_ecma_codegen::Config { minify: false },
      cm: self.cm.clone(),
      comments: None,
      wr: Box::new(JsWriter::new(self.cm.clone(), "\n", wr.lock(), None)),
    };
    emitter.emit_module(node).unwrap();
  }

  pub fn generate_module_graph(this: &Shared<Self>) {
    let norOrExt = Graph::fetch_module(this, &this.entry, None);
    if let NorOrExt::Normal(module) = &norOrExt {
      this.borrow_mut().entry_module.replace(module.clone());
    }
  }

  pub fn fetch_module(this: &Shared<Self>, source: &str, importer: Option<&str>) -> NorOrExt {
    let maybe_id = this.hook_driver.resolve_id(source, importer);
    if let Some(id) = &maybe_id {
      if let Some(module) = this.modules.get(id) {
        module.clone()
      } else {
        let source = this.hook_driver.load(id);
        let module = NorOrExt::Normal(Shared::new(Module::new(ModuleOptions {
          source,
          id: id.to_string(),
          graph: this.clone(),
        })));
        this.borrow_mut().modules.insert(id.clone(), module.clone());
        module
      }
    } else {
      if let Some(module) = this.modules.get(source) {
        module.clone()
      } else {
        let module = NorOrExt::External(Shared::new(ExternalModule {
          name: source.to_owned(),
        }));
        this
          .borrow_mut()
          .modules
          .insert(source.to_owned(), module.clone());
        module
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::types::shared::Shared;

  use super::Graph;
  use std::{env, path::Path};

  // #[test]
  // fn extensiton_test() {
  //   let b = Bundle::new("./foo/bar");
  //   assert!(b.entry.ends_with(".js"));
  // }
  // #[test]
  // fn entry_path_test() {
  //   let b = Bundle::new("./foo/bar");
  //   let left = Path::new(&b.entry).to_path_buf();
  //   let right = Path::join(env::current_dir().unwrap().as_path(), "./foo/bar.js");
  //   assert_eq!(left, right);
  // }

  #[test]
  fn e2e() {
    let b = Graph::new("demo/main.js");
    Graph::build(&Shared::new(b));
    // let m = b.entry_module.borrow();
    // let t = &m.as_ref().unwrap().imports;
    // println!("imports {:?}", t);
  }
}

#[derive(Clone)]
pub enum NorOrExt {
  Normal(Shared<Module>),
  External(Shared<ExternalModule>),
}

impl NorOrExt {
  pub fn is_normal(&self) -> bool {
    if let NorOrExt::Normal(_) = self {
      true
    } else {
      false
    }
  }
  pub fn is_external(&self) -> bool {
    !self.is_normal()
  }
}
