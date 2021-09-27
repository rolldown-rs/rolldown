#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

use napi::{self, CallContext, Env, JsBuffer, JsObject, JsString, Result, Task};

#[cfg(all(
  target_arch = "x86_64",
  not(target_env = "musl"),
  not(target_os = "macos"),
  not(debug_assertions)
))]
#[global_allocator]
static ALLOC: mimalloc_rust::GlobalMiMalloc = mimalloc_rust::GlobalMiMalloc;

#[cfg(all(
  target_os = "macos",
  not(target_arch = "aarch64"),
  not(debug_assertions)
))]
#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

#[cfg(all(target_os = "macos", target_arch = "aarch64", not(debug_assertions)))]
#[global_allocator]
static ALLOC: mimalloc_rust::GlobalMiMalloc = mimalloc_rust::GlobalMiMalloc;

#[module_exports]
fn init(mut exports: JsObject) -> Result<()> {
  exports.create_named_method("rolldown", rolldown)?;
  exports.create_named_method("rolldownSync", rolldown_sync)?;
  Ok(())
}

#[derive(Debug)]
struct Rolldown {
  entry: String,
}

impl Task for Rolldown {
  type Output = Vec<u8>;
  type JsValue = JsBuffer;

  fn compute(&mut self) -> Result<Self::Output> {
    let bundle = rolldown::Bundle::new(self.entry.as_str())
      .map_err(|err| napi::Error::new(napi::Status::GenericFailure, format!("{}", err)))?;
    let mut output = Vec::new();
    bundle
      .generate(&mut output)
      .map_err(|err| napi::Error::new(napi::Status::GenericFailure, format!("{}", err)))?;
    Ok(output)
  }

  fn resolve(self, env: Env, output: Self::Output) -> Result<Self::JsValue> {
    env.create_buffer_with_data(output).map(|v| v.into_raw())
  }
}

#[js_function(1)]
fn rolldown_sync(ctx: CallContext) -> Result<JsBuffer> {
  let entry = ctx.get::<JsString>(0)?.into_utf8()?;
  let bundle = rolldown::Bundle::new(entry.as_str()?)
    .map_err(|err| napi::Error::new(napi::Status::GenericFailure, format!("{}", err)))?;
  let mut output = Vec::new();
  bundle
    .generate(&mut output)
    .map_err(|err| napi::Error::new(napi::Status::GenericFailure, format!("{}", err)))?;
  ctx
    .env
    .create_buffer_with_data(output)
    .map(|v| v.into_raw())
}

#[js_function(1)]
fn rolldown(ctx: CallContext) -> Result<JsObject> {
  let entry = ctx.get::<JsString>(0)?.into_utf8()?;
  ctx
    .env
    .spawn(Rolldown {
      entry: entry.as_str()?.to_owned(),
    })
    .map(|promise| promise.promise_object())
}
