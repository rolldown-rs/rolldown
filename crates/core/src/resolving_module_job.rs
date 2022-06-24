use std::{
  path::Path,
  sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
  },
};

use sugar_path::PathSugar;
use swc_atoms::JsWord;
use swc_ecma_transforms::resolver;
use swc_ecma_visit::VisitMutWith;
use swc_common::Mark;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
  LoadArgs,  Msg, PluginDriver,
  ResolveArgs, VisitedModuleIdentity, module::JsModule, resolve, load, parse_file, DependencyScanner, get_swc_compiler
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
  context: JobContext,
  dependency: Dependency,
  tx: UnboundedSender<Msg>,
  plugin_driver: Arc<PluginDriver>,
}

impl ResolvingModuleJob {
  pub fn new(
    context: JobContext,
    dependency: Dependency,
    tx: UnboundedSender<Msg>,
    plugin_driver: Arc<PluginDriver>,
  ) -> Self {
    context.active_task_count.fetch_add(1, Ordering::SeqCst);

    Self {
      context,
      dependency,
      tx,
      plugin_driver,
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

  pub async fn resolve_module(&mut self) -> anyhow::Result<Option<JsModule>> {
    let id: JsWord = resolve(
      ResolveArgs {
        importer: self.dependency.importer.as_deref(),
        specifier: self.dependency.specifier.as_ref(),
      },
      &self.plugin_driver,
      &mut self.context,
    )
    .await?.into();
    
    tracing::trace!("resolved id {:?}", id);

    self
      .tx
      .send(Msg::DependencyReference(
        self.dependency.importer.clone().into(),
        (self.dependency.specifier.clone(), id.clone().into()),
      ))
      .unwrap();

    if self
      .context
      .visited_module_identity
      .contains(&id)
    {
      return Ok(None);
    }

    self
      .context
      .visited_module_identity
      .insert(id.clone());

    let source_code = load(
      &self.plugin_driver,
      LoadArgs { id: &id },
      &mut self.context,
    )
    .await?;
    // TODO: transform


    let mut ast = parse_file(source_code, &id);

    self.pre_scan_dependencies(&ast, id.clone().into());
  

    let mut scanner = DependencyScanner::default();

    ast.visit_mut_with(&mut scanner);
    
    let top_level_mark = get_swc_compiler().run(|| Mark::new());

    get_swc_compiler().run(|| {
      ast.visit_mut_with(&mut resolver(get_swc_compiler().run(|| Mark::new()), top_level_mark, false));
    });

    let module = JsModule {
      ast,
      id,
      dependecies: scanner.dependencies,
      dyn_dependecies: scanner.dyn_dependencies,
      top_level_mark,
      imports: Default::default(),
      
    };

    tracing::trace!("parsed module {:?}", module);

    Ok(Some(module))


    
    // scan deps

    // let deps = module
    //   .dependencies()
    //   .into_iter()
    //   .map(|dep| Dependency {
    //     importer: Some(uri.clone()),
    //     detail: dep,
    //   })
    //   .collect::<Vec<_>>();

    // tracing::trace!("get deps {:?}", deps);
    // deps.iter().for_each(|dep| {
    //   self.fork(dep.clone());
    // });
  }

  fn fork(&self, dep: Dependency) {
    let task = ResolvingModuleJob::new(
      JobContext {
        module_name: None,
        ..self.context.clone()
      },
      dep,
      self.tx.clone(),
      self.plugin_driver.clone(),
    );

    tokio::task::spawn(async move {
      task.run().await;
    });
  }

  fn pre_scan_dependencies(&self, ast: &swc_ecma_ast::Program, id: JsWord) -> Option<()> {
    let module = ast.as_module()?;
    module.body.iter().filter_map(|stmt| stmt.as_module_decl()).filter_map(|module_decl| {
      match module_decl {
        swc_ecma_ast::ModuleDecl::Import(import) => {
          Some(Dependency { importer: Some(id.clone()), specifier: import.src.value.clone() })
        },
        swc_ecma_ast::ModuleDecl::ExportDecl(_) => todo!(),
        swc_ecma_ast::ModuleDecl::ExportNamed(decl) => {
          decl.src.as_ref().map(|exported| Dependency { importer: Some(id.clone()), specifier: exported.value.clone() })
        },
        // swc_ecma_ast::ModuleDecl::ExportDefaultDecl(_) => todo!(),
        // swc_ecma_ast::ModuleDecl::ExportDefaultExpr(_) => todo!(),
        swc_ecma_ast::ModuleDecl::ExportAll(decl) => {
          Some(Dependency { importer: Some(id.clone()), specifier: decl.src.value.clone() })
        },
        // swc_ecma_ast::ModuleDecl::TsImportEquals(_) => todo!(),
        // swc_ecma_ast::ModuleDecl::TsExportAssignment(_) => todo!(),
        // swc_ecma_ast::ModuleDecl::TsNamespaceExport(_) => todo!(),
        _ => None
    }
    }).for_each(|depenency| {
      self.fork(depenency);
    });

    None
  }
}

#[derive(Debug, Clone)]
pub struct JobContext {
  pub module_name: Option<String>,
  pub(crate) active_task_count: Arc<AtomicUsize>,
  pub(crate) visited_module_identity: VisitedModuleIdentity,
}
