use std::collections::HashMap;

use hashbrown::HashSet;
use swc_atoms::JsWord;
use swc_common::{util::take::Take, FileName};
use tracing::instrument;
use std::fmt::Debug;
use swc_ecma_ast::EsVersion;
use swc_ecma_transforms::{pass::noop, react, hygiene};
use swc_ecma_visit::VisitMutWith;
use swc::config::{self as swc_config, SourceMapsConfig};



use crate::{ModuleGraph, JsModule, Compilation, get_swc_compiler};

#[derive(Debug)]
pub struct Chunk {
  pub id: String,
  pub(crate) module_uris: HashSet<String>,
  pub module_index: HashMap<JsWord, usize>,
  pub entry_uri: String,
}

impl Chunk {
  pub fn new(id: String, entry_uri: String, kind: ChunkKind) -> Self {
    Self {
      id,
      module_uris: Default::default(),
      entry_uri,
      module_index: Default::default(),
    }
  }

  #[instrument]
  pub fn ordered_modules<'a>(&self, module_graph: &'a ModuleGraph) -> Vec<&'a JsModule> {
    let mut ordered = self
      .module_uris
      .iter()
      .filter_map(|uri| module_graph.module_by_id(uri))
      .collect::<Vec<_>>();
    ordered.sort_by_key(|m| self.module_index[&m.id]);
    ordered
  }

  pub fn calc_exec_order(&mut self, module_graph: &ModuleGraph) {
    let entries = [self.entry_uri.clone()];
    let mut visited = HashSet::new();

    let mut next_exec_order = 0;
    for entry in entries {
      let mut stack_visited: HashSet<String> = HashSet::new();
      let mut stack = vec![entry];
      while let Some(module_uri) = stack.pop() {
        if !visited.contains(&module_uri) {
          if stack_visited.contains(module_uri.as_str()) {
            self
              .module_index
              .insert(module_uri.clone().into(), next_exec_order);
            // tracing::debug!(
            //   "module: {:?},next_exec_order {:?}",
            //   module_uri,
            //   next_exec_order
            // );
            next_exec_order += 1;
            visited.insert(module_uri);
          } else {
            stack.push(module_uri.to_string());
            stack_visited.insert(module_uri.to_string());
            stack.append(
              &mut module_graph
                .module_by_id(&module_uri)
                .unwrap()
                .depended_modules(module_graph)
                .into_iter()
                .rev()
                .map(|dep_mod| dep_mod.id.to_string())
                // .cloned()
                .collect(),
            )
          }
        }
      }
    }
  }

  pub fn render(&self, compilation: &Compilation) -> String {
    let mut module = swc_ecma_ast::Module::dummy();
    module.body = self.ordered_modules(&compilation.module_graph).iter().filter_map(|js_mod| js_mod.ast.as_module()).flat_map(|module| module.body.clone()).collect();
    let compiler = get_swc_compiler();
    let output = swc::try_with_handler(compiler.cm.clone(), Default::default(), |handler| {
      let fm = compiler
        .cm
        .new_source_file(FileName::Custom(self.id.to_string()), self.id.to_string());

      let source_map = false;


      let hygiene_visitor = hygiene();
      compiler.process_js_with_custom_pass(
        fm,
        // TODO: It should have a better way rather than clone.
        Some(swc_ecma_ast::Program::Module(module)),
        handler,
        &swc_config::Options {
          config: swc_config::Config {
            jsc: swc_config::JscConfig {
              target: Some(EsVersion::Es2022),
              syntax: Default::default(),
              transform: Some(swc_config::TransformConfig {
                react: react::Options {
                  runtime: Some(react::Runtime::Automatic),
                  ..Default::default()
                },
                ..Default::default()
              })
              .into(),
              ..Default::default()
            },
            inline_sources_content: true.into(),
            // emit_source_map_columns: (!matches!(options.mode, BundleMode::Dev)).into(),
            source_maps: Some(SourceMapsConfig::Bool(source_map)),
            ..Default::default()
          },
          // top_level_mark: Some(bundle_ctx.top_level_mark),
          ..Default::default()
        },
        |_, _| hygiene(),
        |_, _| {
          noop()
        },
      )
    })
    .unwrap();
    output.code
  }
}

#[derive(Debug)]
pub enum ChunkKind {
  Entry { name: String },
  Normal,
  // TODO: support it.
  // Initial,
}

impl ChunkKind {
  pub fn is_entry(&self) -> bool {
    matches!(self, ChunkKind::Entry { .. })
  }
  pub fn is_normal(&self) -> bool {
    matches!(self, ChunkKind::Normal)
  }
}
