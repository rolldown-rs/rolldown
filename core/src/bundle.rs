use std::io::{self, Write};

use swc_common::{BytePos, LineCol};
use swc_ecma_ast::EsVersion;
use swc_ecma_codegen::{text_writer::JsWriter, Node};
use thiserror::Error;

use crate::{graph, module::analyse, types::Shared, utils::plugin_driver::{self, PluginDriver}};

#[derive(Debug, Error)]
pub enum BundleError {
  #[error("{0}")]
  GraphError(crate::graph::GraphError),
  #[error("{0}")]
  IoError(io::Error),
  #[error("No Module found")]
  NoModule,
}

impl From<io::Error> for BundleError {
  fn from(err: io::Error) -> Self {
    Self::IoError(err)
  }
}

impl From<graph::GraphError> for BundleError {
  fn from(err: graph::GraphError) -> Self {
    Self::GraphError(err)
  }
}

#[derive(Clone)]
#[non_exhaustive]
pub struct Bundle {
  pub graph: graph::Graph,
  plugin_driver: Shared<PluginDriver>,
}

impl Bundle {
  pub fn new(entry: &str) -> Result<Self, BundleError> {

    let plugin_driver = PluginDriver::new();

    Ok(Self {
      graph: graph::Graph::new(entry, plugin_driver.clone())?,
      plugin_driver: plugin_driver.clone(),
    })
  }

  // pub fn generate<W: Write>(
  //   self,
  //   w: W,
  //   sm: Option<&mut Vec<(BytePos, LineCol)>>,
  // ) -> Result<(), BundleError> {
  //   let statements = self.graph.build();
  //   statements
  //     .iter()
  //     .filter_map(|s| {
  //       self
  //         .graph
  //         .get_module(&s.module_id)
  //         .into_mod()
  //         .map(|m| (s, m))
  //     })
  //     .for_each(|(s, module)| {
  //       if s.is_export_declaration {
  //         analyse::fold_export_decl_to_decl(&mut s.node.write(), module);
  //       }
  //     });

  //   let mut emitter = swc_ecma_codegen::Emitter {
  //     cfg: swc_ecma_codegen::Config { minify: false },
  //     cm: graph::SOURCE_MAP.clone(),
  //     comments: None,
  //     wr: Box::new(JsWriter::with_target(
  //       graph::SOURCE_MAP.clone(),
  //       "\n",
  //       w,
  //       sm,
  //       EsVersion::latest(),
  //     )),
  //   };
  //   for stmt in statements {
  //     stmt.node.read().emit_with(&mut emitter)?;
  //   }
  //   Ok(())
  // }
}
