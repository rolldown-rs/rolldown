use std::env;

use rolldown::Bundle;

fn main() {
  let mut args = env::args();
  let entry = args.nth(1).unwrap();
  let entry_path = match entry.as_str() {
    "lodash" => "./node_modules/lodash-es/lodash.js",
    "three" => "./benchmark/three.js/src/Three.js",
    _ => panic!("Unknown entry point"),
  };
  let bundle = Bundle::new(entry_path).unwrap();
  let mut output = Vec::new();
  bundle.generate(&mut output, None).unwrap();
}
