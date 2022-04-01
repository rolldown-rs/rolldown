use std::time::Instant;

use rolldown::{
  types::{NormalizedInputOptions, NormalizedOutputOptions},
  RolldownBuild,
};

// use rolldown::graph::GraphContainer;

fn main() {
  let _start = Instant::now();
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
  let rolldown_build = RolldownBuild::new(NormalizedInputOptions {
    input: vec![
      // "./tests/fixtures/tree-shaking4/index.js".to_owned(),
      "./tests/fixtures/tree-shaking3/index.js".to_owned(),
      // "./tests/fixtures/tree-shaking2/index.js".to_owned(),
      // "./tests/fixtures/re_export_default/index.js".to_owned(),
      // "./tests/fixtures/re_export_default/index.js".to_owned(),
      // "./tests/fixtures/namespace/index.js".to_owned(),
      // "../../three.js/src/Three.js".to_owned(),
      // "./tests/fixtures/preact/index.js".to_owned(),
      // "./tests/fixtures/tree-shaking1/index.js".to_owned(),
      // "../node_modules/lodash-es/lodash.js".to_owned(),
      // "../node_modules/lodash-es/array.default.js".to_owned()
    ],
    treeshake: true,
  });
  let output = rolldown_build.write(NormalizedOutputOptions {
    // entry_file_names: "[name].js".to_string(),
    file: Some("./output.js".to_string()),
    // dir: Some("./output.js".to_string()),
    ..Default::default()
  });

  log::info!("output:\n{:#?}", output);
  println!("{}", output[0].get_content());
  // println!("gen() finished in {}", start.elapsed().as_millis());
}
