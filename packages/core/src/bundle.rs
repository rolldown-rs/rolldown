use crate::{
  graph,
  module::{Module, ModuleOptions},
  types::shared::Shared,
};
use path_absolutize::Absolutize;
use std::{
  io::stdout,
  path::{Path, PathBuf},
};
use swc_ecma_codegen::text_writer::JsWriter;

#[derive(Clone)]
pub struct Bundle {
  pub graph: Shared<graph::Graph>,
}
impl Bundle {
  fn new() {}
  pub fn gennerate(&self) {
    let node = graph::Graph::build(&self.graph);
    let wr = stdout();
    let mut emitter = swc_ecma_codegen::Emitter {
      cfg: swc_ecma_codegen::Config { minify: false },
      cm: self.graph.cm.clone(),
      comments: None,
      wr: Box::new(JsWriter::new(self.graph.cm.clone(), "\n", wr.lock(), None)),
    };
    emitter.emit_module(&node).unwrap();
  }
}

#[cfg(test)]
mod tests {
  use crate::{graph::Graph, types::shared::Shared};

  use super::Bundle;

  #[test]
  fn e2e() {
    let g = Graph::new("demo/main.js");
    let bundle = Bundle {
      graph: Shared::new(g),
    };
    bundle.gennerate();
  }
}
