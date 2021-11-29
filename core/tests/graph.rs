use rolldown::Graph;

#[cfg(test)]
mod basic {
  use super::*;
  #[test]
  fn write() {
    let mut graph = Graph::new("./tests/fixtures/dynamic-import/main.js");
    graph.build();

    println!("entry_modules {:#?}", graph.entry_modules)
  }
}
