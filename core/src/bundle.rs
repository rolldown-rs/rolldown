use std::io::{self, Write};
use std::sync::Arc;
use swc_common::{BytePos, LineCol};
use swc_ecma_ast::EsVersion;
use swc_ecma_codegen::{text_writer::JsWriter, Node};
use thiserror::Error;

use crate::graph;
use crate::module::Module;

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
  pub graph: Arc<graph::Graph>,
}

impl Bundle {
  pub fn new(entry: &str) -> Result<Self, BundleError> {
    Ok(Self {
      graph: graph::Graph::new(entry)?,
    })
  }

  pub fn generate<W: Write>(
    self,
    w: W,
    sm: Option<&mut Vec<(BytePos, LineCol)>>,
  ) -> Result<(), BundleError> {
    let statements = Module::expand_all_statements(&self.graph.entry_module, true);
    let mut emitter = swc_ecma_codegen::Emitter {
      cfg: swc_ecma_codegen::Config { minify: false },
      cm: graph::SOURCE_MAP.clone(),
      comments: None,
      wr: Box::new(JsWriter::with_target(
        graph::SOURCE_MAP.clone(),
        "\n",
        w,
        sm,
        EsVersion::latest(),
      )),
    };
    for stmt in statements {
      stmt.node.read().emit_with(&mut emitter)?;
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::Bundle;

  #[test]
  fn e2e() {
    let bundle = Bundle::new("fixtures/main.js").expect("Create bundle failed");
    let mut output = Vec::new();
    let mut sm = Vec::new();
    assert!(bundle.generate(&mut output, Some(&mut sm)).is_ok());
    assert_eq!(
      String::from_utf8(output).expect("Output is not utf8"),
      r#"function add(a, b) {
    return a + b;
}
const noUsed = ()=>{
    return `I'm no used function`;
};
function mul(a, b) {
    let result = 0;
    for(let i = 0; i < a; i++){
        result = add(result, b);
    }
    return result;
}
console.log(mul(8, 9));
"#
    )
  }
}
