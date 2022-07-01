use hashbrown::HashMap;
use std::sync::Mutex;
use swc_common::Mark;

use crate::{get_swc_compiler, ClearMark, ExportRemover, Graph, NormalizedInputOptions, Renamer};
use crate::{shake, ufriend::UFriend, Module, ModuleById};
use ast::{EsVersion, Id, ModuleDecl, ModuleItem, Str};
use hashbrown::HashSet;
use petgraph::unionfind::UnionFind;
use rayon::prelude::*;
use std::fmt::Debug;
use swc::config::{self as swc_config, SourceMapsConfig};
use swc_atoms::JsWord;
use swc_common::{util::take::Take, FileName};
use swc_ecma_transforms::{hygiene, pass::noop, react};
use swc_ecma_visit::{FoldWith, VisitMutWith};
use tracing::instrument;

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

    pub fn prepare(&self, ctx: &mut PrepareContext) {
        self.generate_exports(ctx);
        self.de_conflict(ctx);
    }

    fn generate_exports(&self, ctx: &mut PrepareContext) {
        let entry_module = ctx.modules.get_mut(&self.entry_module_id).unwrap();
        if !entry_module.merged_exports.is_empty() {
            entry_module.generate_exports();
        }
    }

    pub fn de_conflict(&self, ctx: &mut PrepareContext) {
        // modules: &ModuleById, uf: &Mutex<UFriend<Id>>
        let modules = &mut ctx.modules;
        let uf = ctx.uf;

        let mut used_names = HashSet::new();
        let mut id_to_name = HashMap::new();
        // let uf = &mut *uf.lock().unwrap();

        // De-conflict from the entry module to keep namings as simple as possible
        self.ordered_modules(modules)
            .iter()
            .rev()
            .for_each(|module| {
                module.declared_ids.iter().for_each(|id| {
                    let uf = &mut uf.lock().unwrap();
                    uf.add_key(id.clone());
                    let root_id = uf.asset_find_root(id);
                    if let hashbrown::hash_map::Entry::Vacant(e) = id_to_name.entry(root_id.clone())
                    {
                        let original_name = id.0.clone();
                        let mut name = id.0.clone();
                        let mut count = 1;
                        while used_names.contains(&name) {
                            name = format!("{}${}", &original_name, &count).into();
                            count += 1;
                        }
                        e.insert(name.clone());
                        used_names.insert(name);
                    } else {
                    }
                });
            });

        modules.par_values_mut().for_each(|module| {
            let ast = &mut module.ast;
            let mut renamer = Renamer {
                uf: ctx.uf,
                rename_map: &id_to_name,
            };
            ast.as_mut_module().unwrap().body.retain(|module_item| {
                !matches!(module_item, ModuleItem::ModuleDecl(ModuleDecl::Import(_)))
            });
            ast.visit_mut_with(&mut renamer);

            *ast = std::mem::replace(ast, ast::Program::Module(Take::dummy()))
                .fold_with(&mut ExportRemover);
        });

        tracing::debug!("id_to_name {:#?}", id_to_name);
    }

    #[instrument]
    pub fn ordered_modules<'a>(&self, module_by_id: &'a ModuleById) -> Vec<&'a Module> {
        let mut ordered = self
            .module_ids
            .iter()
            .filter_map(|uri| module_by_id.get(uri))
            .collect::<Vec<_>>();
        ordered.sort_by_key(|m| m.exec_order);
        ordered
    }

    pub fn render(&self, graph: &Graph, input_options: &NormalizedInputOptions) -> String {
        let compiler = get_swc_compiler();
        self.ordered_modules(&graph.module_by_id)
            .iter()
            .map(|module| (module, module.ast.as_module().clone().unwrap()))
            .map(|(module, ast)| {
                //     let ast = if input_options.treeshake {
                //         shake(*module, ast.clone(), graph.unresolved_mark)
                //     } else {
                //         ast.clone()
                //     };
                ast.clone()
            })
            .map(|ast| {
                let output =
                    swc::try_with_handler(compiler.cm.clone(), Default::default(), |handler| {
                        let fm = compiler.cm.new_source_file(
                            FileName::Custom(self.id.to_string()),
                            self.id.to_string(),
                        );

                        let source_map = false;

                        compiler.process_js_with_custom_pass(
                            fm,
                            // TODO: It should have a better way rather than clone.
                            Some(ast::Program::Module(ast)),
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
                            |_, _| noop(),
                            |_, _| noop(),
                        )
                    })
                    .unwrap();
                output.code
            })
            .collect::<Vec<_>>()
            .join("\n")
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

pub struct PrepareContext<'a> {
    pub modules: ModuleById,
    pub uf: &'a Mutex<UFriend<Id>>,
    pub unresolved_mark: Mark,
}
