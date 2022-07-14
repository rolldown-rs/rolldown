use rolldown_core::log::enable_tracing_by_env;
use testing_macros::fixture;

use crate::common::test_fixture;
use std::path::{Path, PathBuf};

// TODO: we should split these tests in multiple functions to enable concurrency.
// See https://github.com/swc-project/swc/blob/dc78cb48b928d33197de48dbea0181f8c78d78cd/crates/swc_ecma_transforms_base/tests/fixture.rs#L68
// #[tokio::test]
// async fn js() {
//   let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
//     .join("fixtures")
//     .join("*");

//   let fixture_dir_names = glob::glob(&manifest_dir.to_string_lossy())
//     .unwrap()
//     .into_iter()
//     .filter_map(|path| path.ok())
//     .filter_map(|path| path.file_name().map(|s| s.to_string_lossy().to_string()))
//     .filter(|path| !path.starts_with('.'))
//     .collect::<Vec<_>>();

//   for fixture in &fixture_dir_names {
//     test_fixture(fixture).await;
//   }
// }

#[fixture("./fixtures/*")]
fn js2(path: PathBuf) {
  enable_tracing_by_env();
  tokio::runtime::Runtime::new().unwrap().block_on(test_fixture(&path));
}

