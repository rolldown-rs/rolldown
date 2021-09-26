use std::io::{self, stdout};

use swc_ecma_codegen::text_writer::JsWriter;
use thiserror::Error;

use crate::{graph, types::shared::Shared};

#[derive(Debug, Error)]
pub enum BundleError {
  #[error("{0}")]
  GraphError(crate::graph::GraphError),
  #[error("{0}")]
  IoError(io::Error),
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
  pub graph: Shared<graph::Graph>,
}

impl Bundle {
  pub fn new(graph: Shared<graph::Graph>) -> Self {
    Self { graph }
  }

  pub fn generate(&self) -> Result<(), BundleError> {
    let node = graph::Graph::build(&self.graph)?;
    let wr = stdout();
    let mut emitter = swc_ecma_codegen::Emitter {
      cfg: swc_ecma_codegen::Config { minify: false },
      cm: graph::SOURCE_MAP.clone(),
      comments: None,
      wr: Box::new(JsWriter::new(
        graph::SOURCE_MAP.clone(),
        "\n",
        wr.lock(),
        None,
      )),
    };
    emitter.emit_module(&node)?;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use crate::{graph::Graph, types::shared::Shared};

  use super::Bundle;

  #[test]
  fn e2e() {
    let g = Graph::new("demo/main.js");
    let bundle = Bundle::new(Shared::new(g));
    assert!(bundle.generate().is_ok());
  }
}
