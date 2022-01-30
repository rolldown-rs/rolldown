use rolldown::{graph::Graph, bundle::Bundle};

#[test]
fn basic() {
  // let mut graph = GraphContainer::from_single_entry("./tests/fixtures/basic/main.js".to_owned());
  // let mut graph = GraphContainer::from_single_entry("./tests/fixtures/symbols.js".to_owned());
  // let mut graph = GraphContainer::from_single_entry("../testcase/custom/samples/default-export/main.js".to_owned());
  // let mut graph = GraphContainer::from_single_entry("./tests/fixtures/conflicted/index.js".to_owned());
  let mut graph =
    Graph::from_single_entry("./tests/fixtures/conflicted/index.js".to_owned());
  // let mut graph =
  //   GraphContainer::from_single_entry("../node_modules/lodash-es/lodash.js".to_owned());
  graph.build();
  let mut bundle = Bundle::new(graph);

  let output = bundle.generate();
  insta::assert_snapshot!(output);
}
