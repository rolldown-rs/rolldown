use std::{
  path::Path,
  sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
  },
};

use nodejs_resolver::Resolver;
use rayon::prelude::*;
use swc_atoms::JsWord;
use tracing::instrument;

use crate::{
  CompilerOptions, PluginDriver, Plugin, Stats, ResolvingModuleJob, JobContext, module::JsModule,
};

mod compilation;
pub use compilation::*;

pub struct Compiler {
  pub options: Arc<CompilerOptions>,
  pub compilation: Compilation,
  pub plugin_driver: Arc<PluginDriver>,
}

impl Compiler {
  #[instrument(skip_all)]
  pub fn new(options: CompilerOptions, plugins: Vec<Box<dyn Plugin>>) -> Self {
    let options = Arc::new(options);
    let plugin_driver = PluginDriver::new(
      options.clone(),
      plugins,
      Arc::new(Resolver::new(nodejs_resolver::ResolverOptions {
        // prefer_relative: false,
        extensions: vec![".tsx", ".jsx", ".ts", ".js", ".json"]
          .into_iter()
          .map(|s| s.to_string())
          .collect(),
        alias_fields: vec![String::from("browser")],
        ..Default::default()
      })),
    );

    Self {
      options,
      compilation: Default::default(),
      plugin_driver: Arc::new(plugin_driver),
    }
  }

  // #[instrument(skip_all)]
  pub async fn compile(&mut self) -> anyhow::Result<Stats> {
    // TODO: supports rebuild
    self.compilation = Compilation::new(
      // TODO: use Arc<T> instead
      self.options.clone(),
      self.options.entries.clone(),
      Default::default(),
      Default::default(),
    );

    // self.compilation.
    let active_task_count: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Msg>();

    self
      .compilation
      .options
      .entries
      .clone()
      .into_iter()
      .for_each(|(name, dep)| {
        let task = ResolvingModuleJob::new(
          JobContext {
            module_name: Some(name),
            active_task_count: active_task_count.clone(),
            visited_module_identity: self.compilation.visited_module_id.clone(),
          },
          crate::Dependency { importer: None, specifier: dep.path.into() },
          tx.clone(),
          self.plugin_driver.clone(),
        );

        tokio::task::spawn(async move { task.run().await });
      });

    while active_task_count.load(Ordering::SeqCst) != 0 {
      match rx.recv().await {
        Some(job) => match job {
          Msg::TaskFinished(module) => {
            active_task_count.fetch_sub(1, Ordering::SeqCst);
            self.compilation.module_graph.add_module(*module);
          }
          Msg::TaskCanceled => {
            active_task_count.fetch_sub(1, Ordering::SeqCst);
          }
          Msg::DependencyReference(importer, resolved_uri) => {
            self
              .compilation
              .module_graph
              .add_dependency(importer, resolved_uri);
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

    tracing::debug!("module graph {:#?}", self.compilation.module_graph);

    // self.compilation.calc_exec_order();

    self.compilation.seal();

    self.compilation.chunk_graph.chunks_mut().for_each(|chunk| {
      chunk.calc_exec_order(&self.compilation.module_graph);
    });


    self.compilation.chunk_graph.chunks().for_each(|chunk| {
      let code = chunk.render(&self.compilation);
      std::fs::write(format!("./dist/{}.js", chunk.id), code).unwrap();
    });



    // tracing::debug!("chunk graph {:#?}", self.compilation.chunk_graph);
    // Stream::
    // let assets = self
    //   .compilation
    //   .chunk_graph
    //   .id_to_chunk()
    //   .par_keys()
    //   .flat_map(|chunk_id| {
    //     self.plugin_driver.render_manifest(RenderManifestArgs {
    //       chunk_id,
    //       compilation: &self.compilation,
    //     })
    //   })
    //   .collect::<Vec<_>>();

    // tracing::trace!("assets {:#?}", assets);

    let final_out_dir = {
      let root = Path::new(self.compilation.options.root.as_str());
      let out_dir = Path::new("./dist");
      root.join(out_dir)
    };
    std::fs::create_dir_all(&final_out_dir).unwrap();
    // assets.iter().for_each(|asset| {
    //   std::fs::write(final_out_dir.join(asset.final_filename()), asset.source()).unwrap();
    // });

    Ok(Stats::new(vec![]))
  }

  // #[instrument(skip_all)]
  pub async fn run(&mut self) -> anyhow::Result<Stats> {
    self.compile().await
  }
}

#[derive(Debug)]
pub enum Msg {
  DependencyReference(Option<JsWord>, (JsWord, JsWord)),
  TaskFinished(Box<JsModule>),
  TaskCanceled,
  TaskErrorEncountered(anyhow::Error),
}
