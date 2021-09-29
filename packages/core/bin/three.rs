use rolldown::Bundle;

fn main() {
  let bundle = Bundle::new("./benchmark/three.js/src/Three.js").unwrap();
  let mut output = Vec::new();
  bundle.generate(&mut output, None).unwrap();
}
