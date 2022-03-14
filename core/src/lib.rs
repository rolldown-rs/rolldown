#![deny(clippy::all)]

pub mod bundle;
pub mod chunk;
pub mod external_module;
pub mod graph;
// pub mod linker;
pub mod module;
pub mod scanner;
// pub mod statement;
pub mod renamer;
pub mod types;
pub mod utils;
pub mod worker;

use graph::Graph;
use structs::RolldownOutput;
pub use swc_ecma_ast as ast;
use types::{NormalizedInputOptions, NormalizedOutputOptions};

use crate::bundle::Bundle;

// refactor
pub mod compiler;
pub mod ext;
pub mod plugin_driver;
pub mod statement;
pub mod structs;
pub mod symbol_box;

pub struct RolldownBuild {
  pub graph: Graph,
  // pub input_options: NormalizedInputOptions,
}

impl RolldownBuild {
  pub fn new(options: NormalizedInputOptions) -> Self {
    let mut graph = Graph::new(options);
    graph.build();
    Self { graph }
  }

  pub fn generate(self, options: NormalizedOutputOptions) -> Vec<RolldownOutput> {
    handle_generate_write(false, self.graph, options)
  }

  pub fn write(self, options: NormalizedOutputOptions) -> Vec<RolldownOutput> {
    handle_generate_write(true, self.graph, options)
  }
}

#[inline]
fn handle_generate_write(
  is_write: bool,
  graph: Graph,
  output_options: NormalizedOutputOptions,
) -> Vec<RolldownOutput> {
  if is_write {
    assert!(output_options.dir.is_some() || output_options.file.is_some());
  }
  let mut bundle = Bundle::new(graph, output_options);
  let output = bundle.generate();
  let output = output
    .into_iter()
    .map(|(_, output_chunk)| RolldownOutput::Chunk(output_chunk))
    .collect::<Vec<_>>();

  output.iter().for_each(|output| {
    write_output_file(output, &bundle.output_options);
  });

  output
}

fn write_output_file(output_file: &RolldownOutput, output_options: &NormalizedOutputOptions) {
  let file_name = nodejs_path::resolve!(
    &output_options
      .dir
      .clone()
      .unwrap_or_else(|| nodejs_path::dirname(output_options.file.as_ref().unwrap())),
    output_file.get_file_name()
  );

  std::fs::create_dir_all(nodejs_path::dirname(&file_name)).unwrap();
  log::info!("file_name {}", file_name);
  std::fs::write(file_name, output_file.get_content()).unwrap();
}
