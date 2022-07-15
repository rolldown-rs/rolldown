mod common;

use rolldown_core::log::enable_tracing_by_env;
use testing_macros::fixture;

use crate::common::test_fixture;
use std::path::{Path, PathBuf};

#[fixture("./tests/fixtures/rolldown/*")]
fn js(path: PathBuf) {
  enable_tracing_by_env();
  tokio::runtime::Runtime::new().unwrap().block_on(test_fixture(&path));
}

