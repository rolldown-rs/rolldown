use rolldown::{bundle::Bundle, graph::Graph, types::NormalizedInputOptions};

// use rolldown::graph::GraphContainer;

fn main() {
  env_logger::init();
  // let mut graph = GraphContainer::from_single_entry("./tests/fixtures/preact/index.js".to_owned());
  // let mut graph = GraphContainer::from_single_entry("../../three.js/src/Three.js".to_owned());
  // let mut graph = GraphContainer::from_single_entry("./tests/fixtures/basic/main.js".to_owned());
  // let mut graph = GraphContainer::from_single_entry("./tests/fixtures/symbols.js".to_owned());
  // let mut graph = Graph::from_single_entry("./tests/fixtures/tree-shaking/index.js".to_owned());
  // let mut graph = GraphContainer::new("../testcase/custom/samples/default-export/main.js".to_owned());
  // let mut graph = GraphContainer::from_single_entry("./tests/fixtures/conflicted/index.js".to_owned());
  // let mut graph =
  //   GraphContainer::from_single_entry("./tests/fixtures/inter_module/index.js".to_owned());
  // let mut graph =
  //   GraphContainer::from_single_entry("../node_modules/lodash-es/lodash.js".to_owned());
  let mut graph = Graph::new(NormalizedInputOptions {
    input: vec![
      // "../../three.js/src/Three.js".to_owned(),
      "./tests/fixtures/tree-shaking/index.js".to_owned(),
    ],
    treeshake: true,
    ..Default::default()
  });
  graph.build();
  let mut bundle = Bundle::new(graph);

  let output = bundle.generate();
  std::fs::write("./output.js", output.clone()).unwrap();
  log::debug!("output:\n{}", output);
}
