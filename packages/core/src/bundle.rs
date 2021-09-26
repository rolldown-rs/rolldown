use std::io::{self, Write};

use swc_ecma_codegen::text_writer::JsWriter;
use thiserror::Error;

use crate::{graph, types::shared::Shared};

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
  pub graph: Shared<graph::Graph>,
}

impl Bundle {
  pub fn new(entry: &str) -> Result<Self, BundleError> {
    Ok(Self {
      graph: graph::Graph::build(entry)?,
    })
  }

  pub fn generate<W: Write>(&self, w: W) -> Result<(), BundleError> {
    let node = self
      .graph
      .get_swc_module()
      .ok_or_else(|| BundleError::NoModule)?;
    let mut emitter = swc_ecma_codegen::Emitter {
      cfg: swc_ecma_codegen::Config { minify: false },
      cm: graph::SOURCE_MAP.clone(),
      comments: None,
      wr: Box::new(JsWriter::new(graph::SOURCE_MAP.clone(), "\n", w, None)),
    };
    emitter.emit_module(&node)?;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::Bundle;

  #[test]
  fn e2e() {
    let bundle = Bundle::new("demo/main.js").expect("Create bundle failed");
    let mut output = Vec::new();
    assert!(bundle.generate(&mut output).is_ok());
    assert_eq!(
      String::from_utf8(output).expect("Output is not utf8"),
      r#"export default function add(a, b) {
    return a + b;
};
export default function mul(a, b) {
    let result = 0;
    for(let i = 0; i < a; i++){
        result = add(result, b);
    }
    return result;
};
window.console.log(mul(8, 9));
"#
    )
  }
}
