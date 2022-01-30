use dashmap::DashSet;

use crate::{chunk::Chunk, graph};

#[non_exhaustive]
pub struct Bundle {
  pub graph_container: graph::Graph,
}

impl Bundle {
  pub fn new(graph: graph::Graph) -> Self {
    Self {
      graph_container: graph,
    }
  }

  pub fn generate(&mut self) -> String {
    let entries = DashSet::new();
    self.graph_container.entry_indexs.iter().for_each(|entry| {
      let entry = self.graph_container.graph[*entry].to_owned();
      entries.insert(entry);
    });

    let mut chunk = Chunk {
      order_modules: self
        .graph_container
        .ordered_modules
        .clone()
        .into_iter()
        .map(|idx| self.graph_container.graph[idx].clone())
        .collect(),
      symbol_box: self.graph_container.symbol_box.clone(),
      entries,
      canonical_names: Default::default(),
      exports: Default::default(),
    };

    chunk.render(&mut self.graph_container.module_by_id)
  }
}
