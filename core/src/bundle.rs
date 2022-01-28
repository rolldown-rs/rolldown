use crate::{chunk::Chunk, graph, types::NormalizedInputOptions};

#[non_exhaustive]
pub struct Bundle {
  input_options: NormalizedInputOptions,
  graph_container: Option<graph::GraphContainer>,
}

impl Bundle {
  pub fn new(input_options: NormalizedInputOptions) -> Self {
    Self {
      input_options,
      graph_container: None,
    }
  }

  pub fn build(&mut self) {
    let mut graph = graph::GraphContainer::new(&self.input_options);
    graph.build();
    self.graph_container = Some(graph)
  }

  pub fn generate(&mut self) -> String {
    if let Some(graph) = &mut self.graph_container {
      let mut chunk = Chunk {
        order_modules: graph
          .ordered_modules
          .clone()
          .into_iter()
          .map(|idx| graph.graph[idx].clone())
          .collect(),
        symbol_box: graph.symbol_box.clone(),
      };

      chunk.render(&mut graph.id_to_module)
    } else {
      panic!("You may run build first")
    }
  }
}
