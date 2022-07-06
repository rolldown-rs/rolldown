use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use crate::{
    get_swc_compiler, ufriend::UFriend, ExportRemover, JobContext, Module, NormalizedInputOptions,
    Plugin, PluginDriver, ResolvedId, ResolvingModuleJob, SideEffect,
};
use ast::Id;
use dashmap::DashSet;
use hashbrown::{HashMap, HashSet};
use rayon::iter::ParallelIterator;
use swc_atoms::JsWord;
use swc_common::Mark;
use swc_ecma_visit::VisitMutWith;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct Graph {
    pub options: Arc<NormalizedInputOptions>,
    pub module_by_id: HashMap<JsWord, Module>,
    plugin_driver: Arc<RwLock<PluginDriver>>,
    pub resolved_entries: Vec<JsWord>,
    pub unresolved_mark: Mark,
    pub uf: UFriend<Id>,
}

impl Graph {
    pub fn new(options: Arc<NormalizedInputOptions>, plugins: Vec<Box<dyn Plugin>>) -> Self {
        Self {
            options: options.clone(),
            module_by_id: Default::default(),
            plugin_driver: Arc::new(RwLock::new(PluginDriver::new(options, plugins))),
            resolved_entries: Default::default(),
            unresolved_mark: get_swc_compiler().run(|| Mark::new()),
            uf: UFriend::new(),
        }
    }

    fn add_module(&mut self, module: Module) {
        self.module_by_id.insert(module.id.clone(), module);
    }

    fn sort_modules(&mut self) {
        let mut stack = self
            .resolved_entries
            .iter()
            .filter_map(|dep| self.module_by_id.get(dep))
            .map(|module| module.id.clone())
            .collect::<Vec<_>>();

        let mut dyn_imports = vec![];
        let mut visited: HashSet<JsWord> = HashSet::new();
        let mut sorted: HashSet<JsWord> = HashSet::new();
        let mut next_exec_order = 0;
        while let Some(id) = stack.pop() {
            let module = self.module_by_id.get(&id).unwrap();
            if !visited.contains(&id) {
                visited.insert(id.clone());
                stack.push(id.clone());
                module
                    .depended_modules(&self.module_by_id)
                    .into_iter()
                    .rev()
                    .filter(|module| !visited.contains(&module.id))
                    .for_each(|dep| {
                        stack.push(dep.id.clone());
                    });
                module
                    .dynamic_depended_modules(&self.module_by_id)
                    .into_iter()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .for_each(|dep| {
                        dyn_imports.push(dep.id.clone());
                    });
            } else {
                if !sorted.contains(&id) {
                    sorted.insert(id.clone());
                    self.module_by_id.get_mut(&id).unwrap().exec_order = next_exec_order;
                    next_exec_order += 1;
                }
            }
        }
        stack = dyn_imports.into_iter().rev().collect();
        while let Some(id) = stack.pop() {
            let module = self.module_by_id.get(&id).unwrap();
            if !visited.contains(&id) {
                visited.insert(id.clone());
                stack.push(id.clone());
                module
                    .depended_modules(&self.module_by_id)
                    .into_iter()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .for_each(|dep| {
                        stack.push(dep.id.clone());
                    });
            } else {
                if !sorted.contains(&id) {
                    sorted.insert(id.clone());
                    self.module_by_id.get_mut(&id).unwrap().exec_order = next_exec_order;
                    next_exec_order += 1;
                }
            }
        }
        let mut modules = self.module_by_id.values().collect::<Vec<_>>();
        modules.sort_by_key(|m| m.exec_order);
        tracing::trace!(
            "ordered {:#?}",
            modules.iter().map(|m| &m.id).collect::<Vec<_>>()
        );
    }

    fn link_exports(&mut self, order_modules: &[JsWord]) {
        order_modules.iter().for_each(|module_id| {
            let module = self.module_by_id.get(module_id).unwrap();
            if matches!(module.side_effect, Some(SideEffect::Pending)) {
                let side_effect = module
                    .depended_modules(&self.module_by_id)
                    .iter()
                    .filter_map(|module| module.side_effect)
                    .find(|side_effect| !matches!(side_effect, SideEffect::Pending));
                self.module_by_id.get_mut(&module_id).unwrap().side_effect = side_effect;
            }
        });

        order_modules.into_iter().for_each(|module_id| {
            tracing::trace!("link_exports for {}", module_id);
            let cur_module = self.module_by_id.get(&module_id).unwrap();
            let re_exports = cur_module
                .re_exports
                .iter()
                .map(|(unresolved_module_id, imported_specifier)| {
                    (
                        cur_module
                            .resolved_module_ids
                            .get(&unresolved_module_id)
                            .unwrap()
                            .id
                            .clone(),
                        imported_specifier.clone(),
                    )
                })
                .collect::<Vec<_>>();
            std::mem::drop(cur_module);
            re_exports
                .into_iter()
                .map(|(re_exported_module_id, re_exported_specifier)| {
                    let re_exported_module =
                        self.module_by_id.get_mut(&re_exported_module_id).unwrap();
                    re_exported_specifier
                        .iter()
                        .flat_map(|spec| {
                            if &spec.alias.0 == "*" {
                                re_exported_module
                                    .merged_exports
                                    .clone()
                                    .into_iter()
                                    .collect()
                            } else {
                                let original_id = re_exported_module
                                    .get_exported(&spec.original, &mut self.uf)
                                    .unwrap_or_else(|| panic!("original_id not found: {:?}", spec))
                                    .clone();
                                vec![(spec.alias.0.clone(), original_id.clone())]
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
                .into_iter()
                .for_each(|ids| {
                    let module = self.module_by_id.get_mut(&module_id).unwrap();
                    ids.into_iter().for_each(|(alias, id)| {
                        assert!(!module.merged_exports.contains_key(&alias), "{}", id.0);
                        module.merged_exports.insert(alias, id);
                    });
                });
        });
    }

    fn link_imports(&mut self, order_modules: &[JsWord]) {
        order_modules.into_iter().for_each(|module_id| {
            let cur_module = self.module_by_id.get(&module_id).unwrap();
            let imports = cur_module
                .imports
                .clone()
                .into_iter()
                .map(|(unresolved_module_id, imported_specifier)| {
                    (
                        cur_module
                            .resolved_module_ids
                            .get(&unresolved_module_id)
                            .unwrap()
                            .id
                            .clone(),
                        imported_specifier,
                    )
                })
                .collect::<HashMap<_, _>>();
            std::mem::drop(cur_module);

            imports
                .iter()
                .for_each(|(imported_module_id, imported_specifier)| {
                    let imported_module = self.module_by_id.get_mut(&imported_module_id).unwrap();
                    imported_specifier.iter().for_each(|spec| {
                        let original_id = imported_module
                            .get_exported(&spec.original, &mut self.uf)
                            .unwrap_or_else(|| {
                                panic!(
                                    "module {} has no export {}",
                                    imported_module_id, spec.original
                                )
                            })
                            .clone();
                        self.uf.add_key(spec.alias.clone());
                        self.uf.add_key(original_id.clone());
                        self.uf.union(&spec.alias, &original_id);
                        if &spec.original == "default" || &spec.original == "*" {
                            // There is only one case where `specifier.used` is not a valid varible name.
                            // Special case ` export { default } from ...`
                            if &spec.alias.0 != "default" {
                                imported_module
                                    .suggest_name(spec.original.clone(), spec.alias.0.clone());
                            }
                        }
                    });
                });
        });
    }

    fn link(&mut self) {
        let order_modules = {
            let mut modules = self
                .module_by_id
                .values()
                .map(|module| module.id.clone())
                .collect::<Vec<_>>();
            modules.sort_by_key(|id| self.module_by_id[id].exec_order);
            modules
        };

        self.link_exports(&order_modules);
        self.link_imports(&order_modules);
    }

    fn include_statement(&mut self) {
        let order_modules = {
            let mut modules = self
                .module_by_id
                .values()
                .map(|module| module.id.clone())
                .collect::<Vec<_>>();
            modules.sort_by_key(|id| self.module_by_id[id].exec_order);
            modules
        };

        order_modules.into_iter().for_each(|module_id| {
            let module = self.module_by_id.get(&module_id).unwrap();
            let imports = module
                .imports
                .iter()
                .map(|(unresolved_module_id, sids)| {
                    (
                        module
                            .resolved_module_ids
                            .get(&unresolved_module_id)
                            .unwrap()
                            .clone(),
                        sids.clone(),
                    )
                })
                .collect::<Vec<_>>();
            std::mem::drop(module);
            imports.into_iter().for_each(|(module_id, sids)| {
                let imported_module = self.module_by_id.get_mut(&module_id.id).unwrap();
                sids.into_iter()
                    .for_each(|name| imported_module.mark_used_id(&name.original, &name.alias));
            });
        });
    }

    pub fn generate_exports(&mut self) {
        let uf = Mutex::new(&mut self.uf);
        self.module_by_id.par_values_mut().for_each(|module| {
            get_swc_compiler().run(|| {
                module.generate_namespace_export(&uf);
                module.shim_default_export_expr(&uf);
                module.ast.visit_mut_with(&mut ExportRemover);
            });
        });
        // module.generate_namespace_export(ctx.uf);
        //       get_swc_compiler().run(|| {
        //           module.shim_default_export_expr(ctx.uf);
        //       });
        // if let ast::Program::Module(ast_module) = &mut module.ast {
        //     ast_module.visit_mut_with(&mut ExportRemover);
        // }
    }

    async fn build_graph(&mut self) -> anyhow::Result<()> {
        let active_task_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Msg>();

        let visited_module_id = Arc::new(DashSet::new());

        let mut resolved_ids_for_all_module: HashMap<Option<JsWord>, HashMap<JsWord, ResolvedId>> =
            HashMap::new();

        self.options.input.iter().for_each(|(name, dep)| {
            let task = ResolvingModuleJob::new(
                JobContext {
                    unresolved_mark: self.unresolved_mark,
                    module_name: Some(name.clone()),
                    active_task_count: active_task_count.clone(),
                    visited_module_identity: visited_module_id.clone(),
                },
                crate::Dependency {
                    importer: None,
                    specifier: dep.path.clone().into(),
                },
                tx.clone(),
                self.plugin_driver.clone(),
                true,
            );

            tokio::task::spawn(async move { task.run().await });
        });

        while active_task_count.load(Ordering::SeqCst) != 0 {
            match rx.recv().await {
                Some(job) => match job {
                    Msg::TaskFinished(module) => {
                        active_task_count.fetch_sub(1, Ordering::SeqCst);
                        module
                            .local_exports
                            .values()
                            .cloned()
                            .chain(
                                module
                                    .imports
                                    .values()
                                    .flatten()
                                    .map(|spec| &spec.alias)
                                    .cloned(),
                            )
                            .chain(
                                module
                                    .re_exports
                                    .values()
                                    .flatten()
                                    .map(|spec| &spec.alias)
                                    .cloned(),
                            )
                            .for_each(|id| {
                                self.uf.add_key(id.clone());
                            });
                        self.add_module(*module);
                    }
                    Msg::TaskCanceled => {
                        active_task_count.fetch_sub(1, Ordering::SeqCst);
                    }
                    Msg::DependencyReference(importer, spec, resolved_uri) => {
                        resolved_ids_for_all_module
                            .entry(importer)
                            .or_default()
                            .insert(spec, resolved_uri);
                    }
                    Msg::TaskErrorEncountered(err) => {
                        active_task_count.fetch_sub(1, Ordering::SeqCst);
                        return Err(err);
                    }
                },
                None => {
                    tracing::trace!("All sender is dropped");
                }
            }
        }

        tracing::trace!(
            "resolved_ids_for_all_module {:#?}",
            resolved_ids_for_all_module
        );

        self.resolved_entries = resolved_ids_for_all_module
            .remove(&None)
            .unwrap()
            .into_values()
            .map(|rid| rid.id)
            .collect::<Vec<_>>();

        self.module_by_id.values_mut().for_each(|module| {
            module.resolved_module_ids = resolved_ids_for_all_module
                .remove(&Some(module.id.clone()))
                .unwrap_or_default();
        });

        Ok(())
    }

    pub async fn build(&mut self) -> anyhow::Result<()> {
        self.build_graph().await?;
        self.sort_modules();
        self.link();
        self.generate_exports();
        self.include_statement();
        Ok(())
    }
}

#[derive(Debug)]
pub enum Msg {
    DependencyReference(Option<JsWord>, JsWord, ResolvedId),
    TaskFinished(Box<Module>),
    TaskCanceled,
    TaskErrorEncountered(anyhow::Error),
}
