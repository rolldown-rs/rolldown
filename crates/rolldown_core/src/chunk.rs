use std::collections::HashMap;

use crate::{shake, Module};
use ast::{EsVersion, Str};
use hashbrown::HashSet;
use std::fmt::Debug;
use swc::config::{self as swc_config, SourceMapsConfig};
use swc_atoms::JsWord;
use swc_common::{util::take::Take, FileName};
use swc_ecma_transforms::{hygiene, pass::noop, react};
use swc_ecma_visit::VisitMutWith;
use tracing::instrument;

use crate::{get_swc_compiler, Graph};

#[derive(Debug)]
pub struct Chunk {
    pub id: JsWord,
    pub(crate) module_ids: HashSet<JsWord>,
    // pub module_index: HashMap<JsWord, usize>,
    pub entry_module_id: JsWord,
}

impl Chunk {
    pub fn new(id: JsWord, entry_module_id: JsWord) -> Self {
        Self {
            id,
            entry_module_id,
            module_ids: Default::default(),
        }
    }

    #[instrument]
    pub fn ordered_modules<'a>(&self, module_graph: &'a Graph) -> Vec<&'a Module> {
        let mut ordered = self
            .module_ids
            .iter()
            .filter_map(|uri| module_graph.module_by_id.get(uri))
            .collect::<Vec<_>>();
        ordered.sort_by_key(|m| m.exec_order);
        ordered
    }

    pub fn render(&self, graph: &Graph) -> String {
        let mut module = ast::Module::dummy();
        module.body = self
            .ordered_modules(&graph)
            .iter()
            .map(|js_mod| (js_mod, js_mod.ast.as_module().clone().unwrap()))
            .map(|(module, ast)| shake(*module, ast.clone(), graph.unresolved_mark))
            .flat_map(|module| module.body)
            .collect();
        let compiler = get_swc_compiler();
        let output = swc::try_with_handler(compiler.cm.clone(), Default::default(), |handler| {
            let fm = compiler
                .cm
                .new_source_file(FileName::Custom(self.id.to_string()), self.id.to_string());

            let source_map = false;

            compiler.process_js_with_custom_pass(
                fm,
                // TODO: It should have a better way rather than clone.
                Some(ast::Program::Module(module)),
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
                |_, _| noop(),
            )
        })
        .unwrap();
        output.code
    }
}

// #[derive(Debug)]
// pub enum ChunkKind {
//   Entry { name: String },
//   Normal,
//   // TODO: support it.
//   // Initial,
// }

// impl ChunkKind {
//   pub fn is_entry(&self) -> bool {
//     matches!(self, ChunkKind::Entry { .. })
//   }
//   pub fn is_normal(&self) -> bool {
//     matches!(self, ChunkKind::Normal)
//   }
// }

#[derive(Debug)]
pub struct OutputChunk {
  pub code: String,
  pub filename: String,
}
