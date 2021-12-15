use std::collections::HashMap;

use crate::{chunk::Chunk, graph, types::Shared, Module};

// #[derive(Debug, Error)]
// pub enum BundleError {
//   #[error("{0}")]
//   GraphError(crate::graph::GraphError),
//   #[error("{0}")]
//   IoError(io::Error),
//   #[error("No Module found")]
//   NoModule,
// }

// impl From<io::Error> for BundleError {
//   fn from(err: io::Error) -> Self {
//     Self::IoError(err)
//   }
// }

// impl From<graph::GraphError> for BundleError {
//   fn from(err: graph::GraphError) -> Self {
//     Self::GraphError(err)
//   }
// }

#[derive(Clone)]
#[non_exhaustive]
pub struct Bundle {
  pub graph: graph::Graph,
}

impl Bundle {
  pub fn new(graph: graph::Graph) -> Self {
    Self { graph }
  }

  pub fn generate(&self) {

    let _chunks = self.generate_chunks();
  }

  pub fn generate_chunks(&self) -> Vec<Chunk> {
    let chunks = vec![];
    let _chunk_by_module: HashMap<Shared<Module>, Chunk> = HashMap::default();

    chunks
  }

  pub fn add_manual_chunks(&self) -> HashMap<Shared<Module>, String> {
    let manual_chunk_alias_by_entry = HashMap::default();

    manual_chunk_alias_by_entry
  }
}
