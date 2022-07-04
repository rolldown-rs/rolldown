#![feature(test)]

extern crate test;

use test::Bencher;

// use core::log::enable_tracing_by_env;
use hashbrown::HashMap;
use rolldown::rolldown;
use rolldown_core::NormalizedInputOptions;
use std::path::Path;
use sugar_path::PathSugar;

async fn run() {
    let mut rolldown_build = rolldown(NormalizedInputOptions {
        input: HashMap::from([("main".to_string(), "./lodash.js".to_string().into())]),
        root: Path::new("../../examples/lodash-es")
            .resolve()
            .to_string_lossy()
            .to_string(),
        ..Default::default()
    });

    rolldown_build.generate(Default::default()).await.unwrap();
}

#[bench]
fn bench_lodash(b: &mut Bencher) {
    b.iter(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                run().await;
            })
    });
}
