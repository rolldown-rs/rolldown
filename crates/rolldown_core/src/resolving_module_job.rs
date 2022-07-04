use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use dashmap::DashSet;
use swc_atoms::JsWord;
use swc_common::{Mark, DUMMY_SP};
use swc_ecma_transforms::resolver;
use swc_ecma_utils::quote_ident;
use swc_ecma_visit::VisitMutWith;
use tokio::sync::{mpsc::UnboundedSender, RwLock};

use crate::{
    get_swc_compiler, load, parse_file, resolve, LoadArgs, Module, Msg, PluginDriver, ResolveArgs,
    Scanner,
};

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Dependency {
    /// Uri of importer module
    pub importer: Option<JsWord>,
    pub specifier: JsWord,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum ResolveKind {
    Import,
    Require,
    DynamicImport,
    AtImport,
    AtImportUrl,
}

pub struct ResolvingModuleJob {
    is_entry: bool,
    context: JobContext,
    dependency: Dependency,
    tx: UnboundedSender<Msg>,
    plugin_driver: Arc<RwLock<PluginDriver>>,
}

impl ResolvingModuleJob {
    pub fn new(
        context: JobContext,
        dependency: Dependency,
        tx: UnboundedSender<Msg>,
        plugin_driver: Arc<RwLock<PluginDriver>>,
        is_entry: bool,
    ) -> Self {
        context.active_task_count.fetch_add(1, Ordering::SeqCst);

        Self {
            context,
            dependency,
            tx,
            plugin_driver,
            is_entry,
        }
    }
    pub async fn run(mut self) {
        match self.resolve_module().await {
            Ok(maybe_module) => {
                if let Some(module) = maybe_module {
                    self.send(Msg::TaskFinished(Box::new(module)));
                } else {
                    self.send(Msg::TaskCanceled);
                }
            }
            Err(err) => self.send(Msg::TaskErrorEncountered(err)),
        }
    }

    pub fn send(&self, msg: Msg) {
        if let Err(err) = self.tx.send(msg) {
            tracing::trace!("fail to send msg {:?}", err)
        }
    }

    pub async fn resolve_module(&mut self) -> anyhow::Result<Option<Module>> {
        let id = {
            resolve(
                ResolveArgs {
                    importer: self.dependency.importer.as_deref(),
                    specifier: self.dependency.specifier.as_ref(),
                },
                &*self.plugin_driver.read().await,
                &mut self.context,
            )
            .await?
        };

        tracing::trace!("resolved id {:?}", id);

        self.send(Msg::DependencyReference(
            self.dependency.importer.clone().into(),
            self.dependency.specifier.clone(),
            id.clone(),
        ));

        if self.context.visited_module_identity.contains(&id.id) {
            return Ok(None);
        }

        self.context.visited_module_identity.insert(id.id.clone());

        let source_code = load(
            &*self.plugin_driver.read().await,
            LoadArgs { id: &id.id },
            &mut self.context,
        )
        .await?;
        // TODO: transform

        let mut ast = parse_file(source_code, &id.id);

        self.pre_scan_dependencies(&ast, id.id.clone().into());

        let top_level_mark = get_swc_compiler().run(|| Mark::new());

        get_swc_compiler().run(|| {
            ast.visit_mut_with(&mut resolver(
                self.context.unresolved_mark,
                top_level_mark,
                false,
            ));
        });

        let mut scanner = Scanner {
            top_level_mark,
            ..Default::default()
        };

        get_swc_compiler().run(|| {
            ast.visit_mut_with(&mut scanner);
            // scanner.local_exports.insert(
            //     "*".into(),
            //     quote_ident!(DUMMY_SP.apply_mark(Mark::new()), "*").to_id(),
            // );
        });

        let module = Module {
            exec_order: usize::MAX,
            ast,
            id: id.id,
            top_level_mark,
            imports: scanner.imports,
            re_exports: scanner.re_exports,
            local_exports: scanner.local_exports.clone(),
            merged_exports: scanner.local_exports,
            side_effect: scanner.side_effect,
            resolved_module_ids: Default::default(),
            dependencies: scanner.dependencies,
            dyn_dependencies: scanner.dyn_dependencies,
            included: true,
            used_exported_id: Default::default(),
            local_binded_ids: scanner
                .declared_ids
                .into_iter()
                .map(|id| (id.0.clone(), id))
                .collect(),
            suggested_names: Default::default(),
            is_user_defined_entry: self.is_entry,
            // used_exports: Default::default(),
        };

        tracing::trace!("parsed module {:#?}", module);

        Ok(Some(module))
    }

    fn fork(&self, dep: Dependency, is_entry: bool) {
        let task = ResolvingModuleJob::new(
            JobContext {
                module_name: None,
                ..self.context.clone()
            },
            dep,
            self.tx.clone(),
            self.plugin_driver.clone(),
            is_entry,
        );

        tokio::task::spawn(async move {
            task.run().await;
        });
    }

    fn pre_scan_dependencies(&self, ast: &ast::Program, id: JsWord) -> Option<()> {
        let module = ast.as_module()?;
        module
            .body
            .iter()
            .filter_map(|stmt| stmt.as_module_decl())
            .filter_map(|module_decl| match module_decl {
                ast::ModuleDecl::Import(import) => Some(Dependency {
                    importer: Some(id.clone()),
                    specifier: import.src.value.clone(),
                }),
                ast::ModuleDecl::ExportNamed(decl) => {
                    decl.src.as_ref().map(|exported| Dependency {
                        importer: Some(id.clone()),
                        specifier: exported.value.clone(),
                    })
                }
                ast::ModuleDecl::ExportAll(decl) => Some(Dependency {
                    importer: Some(id.clone()),
                    specifier: decl.src.value.clone(),
                }),
                _ => None,
            })
            .for_each(|depenency| {
                self.fork(depenency, false);
            });

        None
    }
}

#[derive(Debug, Clone)]
pub struct JobContext {
    pub module_name: Option<String>,
    pub(crate) active_task_count: Arc<AtomicUsize>,
    pub(crate) visited_module_identity: Arc<DashSet<JsWord>>,
    pub(crate) unresolved_mark: Mark,
}
