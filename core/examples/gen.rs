use rolldown::plugins::node_resolve::NodeResolve;
use rolldown::types::NormalizedInputOptions;
use rolldown::{bundle::Bundle, graph::GraphContainer};
use std::sync::{Arc, Mutex};

// use rolldown::graph::GraphContainer;

fn main() {
  env_logger::init();
  // let mut graph = GraphContainer::from_single_entry("./tests/fixtures/preact/index.js".to_owned());
  // let mut graph = GraphContainer::new("./tests/fixtures/basic/main.js".to_owned());
  // let mut graph = GraphContainer::new("./tests/fixtures/symbols.js".to_owned());
  // let mut graph = GraphContainer::new("../testcase/custom/samples/default-export/main.js".to_owned());
  // let mut graph = GraphContainer::new("./tests/fixtures/conflicted/index.js".to_owned());
  // let mut graph =
  //   GraphContainer::from_single_entry("./tests/fixtures/inter_module/index.js".to_owned());
  // let mut graph =
  //   GraphContainer::from_single_entry("./tests/fixtures/inter_module/index.js".to_owned());
  // let mut graph =
  //   GraphContainer::from_single_entry("../node_modules/lodash-es/lodash.js".to_owned());
  // graph.build();
  let mut bundle = Bundle::new(NormalizedInputOptions {
    input: vec!["./tests/fixtures/inter_module/index.js".to_owned()],
    plugins: Arc::new(Mutex::new(vec![Box::new(NodeResolve {})])),
    ..NormalizedInputOptions::default()
  });

  bundle.build();

  let output = bundle.generate();
  std::fs::write("./output.js", output.clone()).unwrap();
  println!("output {}", output);
}
