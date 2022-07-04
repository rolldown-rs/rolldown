use hashbrown::HashMap;

use swc_common::Mark;

use crate::{get_swc_compiler, ExportRemover, Graph, NormalizedInputOptions, Renamer};
use crate::{ufriend::UFriend, Module, ModuleById};
use ast::{Id, ModuleDecl, ModuleItem};
use hashbrown::HashSet;

use rayon::prelude::*;
use std::fmt::Debug;

use swc_atoms::JsWord;
use swc_common::{util::take::Take};

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
      ctx.modules.par_values_mut().for_each(|module| {
        module.generate_namespace_export(ctx.uf);
        get_swc_compiler().run(|| {
          module.shim_default_export_expr(ctx.uf);
        });
        if let ast::Program::Module(ast_module) = &mut module.ast {
          *ast_module = ast_module.take().fold_with(&mut ExportRemover);
        }
      });
        let entry_module = ctx.modules.get_mut(&self.entry_module_id).unwrap();
        entry_module.generate_exports();
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
                module.local_binded_ids.values().for_each(|id| {
                    let root_id = uf.find_root(id).unwrap();
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

    pub fn render(&self, graph: &Graph, _input_options: &NormalizedInputOptions) -> String {
        self.ordered_modules(&graph.module_by_id)
            .iter()
            .map(|module| module.render())
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
    pub uf: &'a UFriend<Id>,
    pub unresolved_mark: Mark,
}
