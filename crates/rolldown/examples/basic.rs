// use core::log::enable_tracing_by_env;
use hashbrown::HashMap;
use rolldown::rolldown;
use rolldown_core::{NormalizedInputOptions, log::enable_tracing_by_env};
use std::path::Path;
use sugar_path::PathSugar;

#[tokio::main]
async fn main() {
    // let guard = log::enable_tracing_by_env_with_chrome_layer();
    enable_tracing_by_env();
    let mut compiler = rolldown(NormalizedInputOptions {
        input: HashMap::from([("main".to_string(), "./index.js".to_string().into())]),
        root: Path::new("./crates/rolldown/fixtures/basic-import")
          .resolve()
          .to_string_lossy()
          .to_string(),
        ..Default::default()
    });
    // let mut compiler = rolldown(NormalizedInputOptions {
    //     input: HashMap::from([("main".to_string(), "./index.js".to_string().into())]),
    //     root: Path::new("./crates/rolldown/fixtures/circle")
    //       .resolve()
    //       .to_string_lossy()
    //       .to_string(),
    //     ..Default::default()
    // });

    compiler.write().await.unwrap();

    // if let Some(g) = guard {
    //   g.flush()
    // }
}
