#![deny(clippy::all)]

#[macro_use]
extern crate serde_derive;

use napi::bindgen_prelude::*;
use napi_derive::napi;

#[derive(Debug)]
pub struct Rolldown {
  entry: String,
  _options: RolldownOptions,
}

#[derive(Debug, Deserialize)]
struct RolldownOptions {
  #[serde(default)]
  _sourcemap: bool,
}

#[napi]
impl Task for Rolldown {
  type Output = String;
  type JsValue = String;

  fn compute(&mut self) -> Result<Self::Output> {
    let mut graph = rolldown::graph::Graph::from_single_entry(self.entry.clone());
    graph.build();
    let mut bundle = rolldown::bundle::Bundle::new(graph, Default::default());
    let generated = bundle.generate();
    Ok(generated.values().next().unwrap().code.clone())
  }

  fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
    Ok(output)
  }
}

#[napi]
pub fn rolldown(entry: String, config: Buffer) -> Result<AsyncTask<Rolldown>> {
  let config_slice: &[u8] = &config;
  let options: RolldownOptions = serde_json::from_slice(config_slice)
    .map_err(|err| napi::Error::new(napi::Status::InvalidArg, format!("{}", err)))?;

  Ok(AsyncTask::new(Rolldown {
    entry,
    _options: options,
  }))
}
