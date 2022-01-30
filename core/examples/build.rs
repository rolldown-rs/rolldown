use rolldown::graph::Graph;

// use rolldown::graph::GraphContainer;

fn main() {
  // let mut graph = GraphContainer::new("./tests/fixtures/preact/index.js".to_owned());
  // let mut graph = GraphContainer::new("./tests/fixtures/basic/main.js".to_owned());
  // let mut graph = GraphContainer::new("./tests/fixtures/symbols.js".to_owned());
  let mut graph = Graph::from_single_entry("../node_modules/lodash-es/lodash.js".to_owned());
  // let mut graph = GraphContainer::new("./tests/fixtures/conflicted/index.js".to_owned());
  // let mut graph = GraphContainer::new("./tests/fixtures/inter_module/index.js".to_owned());
  graph.build();
}
