use rolldown::{types::NormalizedInputOptions, Graph};

fn main() {
  let o = NormalizedInputOptions {
    input: vec![(None, "./tests/fixtures/dynamic-import/main.js".to_owned())],
    ..NormalizedInputOptions::default()
  };
  let mut graph = Graph::new(o);
  graph.build();

  println!("entry_modules {:#?}", graph.entry_modules)
}
