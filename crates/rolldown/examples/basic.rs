use core::log::enable_tracing_by_env;
use std::{collections::HashMap, path::Path};


use rolldown::rolldown;
use sugar_path::PathSugar;

#[tokio::main]
async fn main() {
  // let guard = log::enable_tracing_by_env_with_chrome_layer();
  enable_tracing_by_env();
  let mut compiler = rolldown(
    core::CompilerOptions {
      entries: HashMap::from([("main".to_string(), "./src/index.js".to_string().into())]),
      root: Path::new("./examples/basic")
        .resolve()
        .to_string_lossy()
        .to_string(),
      ..Default::default()
    },
  );

  compiler.build().await.unwrap();

  // if let Some(g) = guard {
  //   g.flush()
  // }
}
