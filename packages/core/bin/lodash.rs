use rolldown::Bundle;

fn main() {
  let bundle = Bundle::new("../../node_modules/lodash-es/lodash.js").unwrap();
  let mut output = Vec::new();
  bundle.generate(&mut output, None).unwrap();
}
