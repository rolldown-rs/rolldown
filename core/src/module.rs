use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use ahash::RandomState;
use rayon::prelude::*;
use swc_common::{
  errors::{ColorConfig, Handler},
  FileName,
};
use swc_ecma_ast::{ClassDecl, Decl, DefaultDecl, EsVersion, FnDecl, ModuleDecl, ModuleItem, Stmt};
use swc_ecma_parser::TsConfig;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};

use crate::graph;
use crate::graph::{Graph, ModOrExt};
use crate::statement::Statement;

const PAR_THRESHOLD: usize = 8;

macro_rules! stmt_auto_par {
  ($stmt:ident) => {{
    let defines = $stmt.scope.defines.read();
    if defines.len() >= PAR_THRESHOLD {
      defines.par_iter().map(|s| s.clone()).collect()
    } else {
      defines.iter().map(|s| s.clone()).collect()
    }
  }};
}

macro_rules! expand_module_in_statement {
  ($statement:ident, $module:expr) => {{
    let stmt_read_lock = $statement.node.read();
    let statement: &ModuleItem = &stmt_read_lock;
    if let ModuleItem::ModuleDecl(module_decl) = statement {
      match module_decl {
        ModuleDecl::Import(import_decl) => {
          if let Ok(ModOrExt::Mod(m)) = Graph::fetch_module(
            &$module.get_graph(),
            &import_decl.src.value.to_string(),
            Some(&$module.id),
          ) {
            return Module::expand_all_statements(&m, false);
          };
          return vec![];
        }
        ModuleDecl::ExportNamed(node) => {
          // export { foo } from './foo'
          // export { foo as foo2 } from './foo'
          // export * as foo from './foo'
          if let Some(src) = &node.src {
            if let Ok(ModOrExt::Mod(m)) = Graph::fetch_module(
              &$module.get_graph(),
              &src.value.to_string(),
              Some(&$module.id),
            ) {
              return Module::expand_all_statements(&m, false);
            } else {
              return vec![];
            }
          } else {
            // skip `export { foo, bar, baz }`
            return vec![];
          }
        }
        ModuleDecl::ExportDecl(export_decl) => {
          let export = export_decl.decl.clone();
          drop(stmt_read_lock);
          let mut stmt_write_lock = $statement.node.write();
          *stmt_write_lock = ModuleItem::Stmt(Stmt::Decl(export));
          drop(stmt_write_lock);
        }
        // remove `export` from `export default class Foo {...}`
        ModuleDecl::ExportDefaultDecl(export_decl) => {
          if let DefaultDecl::Class(node) = &export_decl.decl {
            let ident = node.ident.clone().unwrap();
            let class = node.class.clone();
            drop(stmt_read_lock);
            let mut stmt_write_lock = $statement.node.write();
            *stmt_write_lock = ModuleItem::Stmt(Stmt::Decl(Decl::Class(ClassDecl {
              // TODO: fix case like `export default class {}`
              ident,
              declare: false,
              class,
            })));
          } else if let DefaultDecl::Fn(node) = &export_decl.decl {
            // TODO: fix case like `export default function {}`
            let ident = node.ident.clone().unwrap();
            let function = node.function.clone();
            drop(stmt_read_lock);
            let mut stmt_write_lock = $statement.node.write();
            *stmt_write_lock = ModuleItem::Stmt(Stmt::Decl(Decl::Fn(FnDecl {
              ident,
              declare: false,
              function,
            })));
          }
        }
        _ => {}
      }
    }

    if $statement.is_included.load(Ordering::Acquire) {
      vec![]
    } else {
      $statement.is_included.store(true, Ordering::Release);
      vec![$statement.clone()]
    }
  }};
}

#[derive(Clone)]
pub struct Module {
  pub source: String,
  graph: *const Graph,
  pub statements: Vec<Arc<Statement>>,
  pub id: String,
  pub imports: HashMap<String, ImportDesc, RandomState>,
  pub exports: HashMap<String, ExportDesc, RandomState>,
  pub defines: HashSet<String, RandomState>,
}

unsafe impl Sync for Module {}
unsafe impl Send for Module {}

impl Module {
  pub(crate) fn empty() -> Self {
    Self {
      source: "".to_owned(),
      id: "".to_owned(),
      graph: std::ptr::null(),
      statements: vec![],
      imports: HashMap::default(),
      exports: HashMap::default(),
      defines: HashSet::default(),
    }
  }

  pub fn new(source: String, id: String, graph: &Arc<Graph>) -> Result<Self, ()> {
    let ast = Module::get_ast(source.clone(), id.clone());
    let statements = ast?
      .body
      .into_par_iter()
      .map(|node| Arc::new(Statement::new(node, id.clone())))
      .collect::<Vec<Arc<Statement>>>();

    let defines = statements
      .par_iter()
      .map(|stmt| stmt_auto_par!(stmt))
      .collect();

    Ok(Module {
      statements,
      source,
      id,
      graph: Arc::as_ptr(graph),
      imports: HashMap::default(),
      exports: HashMap::default(),
      defines,
    })
  }

  fn get_ast(source: String, filename: String) -> Result<swc_ecma_ast::Module, ()> {
    let handler = Handler::with_tty_emitter(
      ColorConfig::Auto,
      true,
      false,
      Some(graph::SOURCE_MAP.clone()),
    );
    let p = Path::new(filename.as_str());
    let fm = graph::SOURCE_MAP.new_source_file(FileName::Custom(filename.clone()), source);

    let ts_config = TsConfig {
      dynamic_import: true,
      decorators: false,
      import_assertions: true,
      tsx: p
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext == "tsx" || ext == "jsx")
        .unwrap_or(false),
      ..Default::default()
    };

    let lexer = Lexer::new(
      // We want to parse ecmascript
      Syntax::Typescript(ts_config),
      // JscTarget defaults to es5
      EsVersion::latest(),
      StringInput::from(fm.as_ref()),
      None,
    );

    let mut parser = Parser::new_from(lexer);

    parser.take_errors().into_iter().for_each(|e| {
      e.into_diagnostic(&handler).emit();
    });

    parser.parse_module().map_err(|e| {
      // Unrecoverable fatal error occurred
      e.into_diagnostic(&handler).emit()
    })
  }

  pub fn de_conflict(&self, statements: &[Statement]) {
    // name => module_id
    let mut definers = HashMap::new();
    // conflict names
    let mut conflicts = HashSet::new();
    statements.iter().for_each(|stmt| {
      stmt.defines.iter().for_each(|name| {
        if definers.contains_key(name) {
          conflicts.insert(name.clone());
        } else {
          definers.insert(name.clone(), stmt.module_id.clone());
        }
      });
    });
  }

  #[inline]
  pub fn expand_all_statements(&self, _is_entry_module: bool) -> Vec<Arc<Statement>> {
    self
      .statements
      .par_iter()
      .flat_map(|s| expand_module_in_statement!(s, self))
      .collect()
  }

  #[inline]
  fn get_graph(&self) -> Arc<Graph> {
    unsafe {
      Arc::increment_strong_count(self.graph);
      Arc::from_raw(self.graph)
    }
  }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ImportDesc {
  source: String,
  name: String,
  local_name: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ExportDesc {
  name: String,
  local_name: String,
}
