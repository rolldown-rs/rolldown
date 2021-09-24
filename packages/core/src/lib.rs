#[macro_use]
extern crate napi_derive;
extern crate swc_common;
extern crate swc_ecma_parser;

use napi::{CallContext, Env, JsObject, JsUndefined, Result, Task};
mod ast;
mod bundle;
mod external_module;
mod graph;
mod helper;
mod hook_driver;
mod module;
mod statement;
mod types;

#[cfg(all(
  target_arch = "x86_64",
  not(target_env = "musl"),
  not(debug_assertions)
))]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[module_exports]
fn init(mut exports: JsObject) -> Result<()> {
  exports.create_named_method("rolldown", rolldown)?;
  Ok(())
}

#[derive(Debug)]
struct Rolldown {}

impl Task for Rolldown {
  type Output = ();
  type JsValue = JsUndefined;

  fn compute(&mut self) -> Result<Self::Output> {
    println!("Do nothing");
    Ok(())
  }

  fn resolve(self, env: Env, _output: Self::Output) -> Result<Self::JsValue> {
    env.get_undefined()
  }
}

#[js_function(1)]
fn rolldown(ctx: CallContext) -> Result<JsObject> {
  ctx
    .env
    .spawn(Rolldown {})
    .map(|promise| promise.promise_object())
}
