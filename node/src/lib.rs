#![deny(clippy::all)]

// #[macro_use]
extern crate napi_derive;
// #[macro_use]
extern crate serde_derive;



// #[cfg(all(
//   not(debug_assertions),
//   not(all(target_os = "windows", target_arch = "aarch64")),
//   not(all(target_os = "linux", target_arch = "aarch64", target_env = "musl")),
// ))]
// #[global_allocator]
// static ALLOC: mimalloc_rust::GlobalMiMalloc = mimalloc_rust::GlobalMiMalloc;

// #[module_exports]
// fn init(mut exports: JsObject) -> Result<()> {
//   exports.create_named_method("rolldown", rolldown)?;
//   Ok(())
// }

// #[derive(Debug)]
// struct Rolldown {
//   entry: String,
//   options: RolldownOptions,
// }

// #[derive(Debug, Deserialize)]
// struct RolldownOptions {
//   #[serde(default)]
//   sourcemap: bool,
// }

// impl Task for Rolldown {
//   type Output = (Vec<u8>, Vec<(BytePos, LineCol)>);
//   type JsValue = JsObject;

//   fn compute(&mut self) -> Result<Self::Output> {
//     let bundle = rolldown::Bundle::new(self.entry.as_str())
//       .map_err(|err| napi::Error::new(napi::Status::GenericFailure, format!("{}", err)))?;
//     let mut output = Vec::with_capacity(1024 * 1024 * 100);
//     let mut sm = Vec::new();
//     bundle
//       .generate(
//         &mut output,
//         if self.options.sourcemap {
//           Some(&mut sm)
//         } else {
//           None
//         },
//       )
//       .map_err(|err| napi::Error::new(napi::Status::GenericFailure, format!("{}", err)))?;
//     Ok((output, sm))
//   }

//   fn resolve(self, env: Env, output: Self::Output) -> Result<Self::JsValue> {
//     let mut obj = env.create_object()?;
//     obj.set_named_property(
//       "code",
//       env
//         .create_buffer_with_data(output.0)
//         .map(|v| v.into_raw())?,
//     )?;
//     obj.set_named_property("map", env.get_null()?)?;
//     Ok(obj)
//   }
// }

// #[js_function(2)]
// fn rolldown(ctx: CallContext) -> Result<JsObject> {
//   let entry = ctx.get::<JsString>(0)?.into_utf8()?;
//   let config = ctx.get::<JsBuffer>(1)?.into_value()?;
//   let config_slice: &[u8] = &config;
//   let options: RolldownOptions = serde_json::from_slice(config_slice)
//     .map_err(|err| napi::Error::new(napi::Status::InvalidArg, format!("{}", err)))?;

//   ctx
//     .env
//     .spawn(Rolldown {
//       entry: entry.as_str()?.to_owned(),
//       options,
//     })
//     .map(|promise| promise.promise_object())
// }
