use rolldown::{bundle::Bundle, graph::GraphContainer};

// use rolldown::graph::GraphContainer;

fn main() {
  env_logger::init();
  // let mut graph = GraphContainer::new("./tests/fixtures/preact/index.js".to_owned());
  // let mut graph = GraphContainer::new("./tests/fixtures/basic/main.js".to_owned());
  // let mut graph = GraphContainer::new("./tests/fixtures/symbols.js".to_owned());
  // let mut graph = GraphContainer::new("../testcase/custom/samples/default-export/main.js".to_owned());
  // let mut graph = GraphContainer::new("./tests/fixtures/conflicted/index.js".to_owned());
  let mut graph = GraphContainer::new("./tests/fixtures/inter_module/index.js".to_owned());
  graph.build();
  let mut bundle = Bundle::new(graph);

  let output = bundle.generate();
  std::fs::write("./output.js", output.clone()).unwrap();
  println!("output {}", output);
}
