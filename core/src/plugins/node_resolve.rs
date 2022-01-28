use std::{collections::HashSet, path::Path};

use once_cell::sync::Lazy;

use crate::{ext::StrExt, plugin_driver::Plugin, types::ResolvedId, utils::is_external_module};

pub struct NodeResolve {}

impl Plugin for NodeResolve {
  fn get_name(&self) -> &'static str {
    "node-resolve"
  }

  fn resolve_id(&mut self, source: &str, importer: Option<&str>) -> crate::types::ResolveIdResult {
    if importer.is_some() && is_external_module(source) {
      let result = {
        let normalized_source = source.replace("node:", "");
        if BUILTIN_MODULES.contains(normalized_source.as_str()) {
          ResolvedId::new(normalized_source, Some(true))
        } else {
          let raw_id =
            node_resolve::resolve_from(normalized_source.as_str(), importer.unwrap().as_path());
          log::debug!("resolving external module {:#?}", normalized_source);
          match raw_id {
            Ok(id) => {
              let file: &Path = id.as_ref();
              // External should be judged based on `external options`
              ResolvedId::new(file.to_string_lossy().to_string(), Some(false))
            }
            Err(_) => panic!("Module {} is not exist.", normalized_source),
          }
        }
      };
      Some(result)
    } else {
      None
    }
  }
}

// from require("module").builtinModules
static BUILTIN_MODULES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
  HashSet::from([
    "_http_agent",
    "_http_client",
    "_http_common",
    "_http_incoming",
    "_http_outgoing",
    "_http_server",
    "_stream_duplex",
    "_stream_passthrough",
    "_stream_readable",
    "_stream_transform",
    "_stream_wrap",
    "_stream_writable",
    "_tls_common",
    "_tls_wrap",
    "assert",
    "async_hooks",
    "buffer",
    "child_process",
    "cluster",
    "console",
    "constants",
    "crypto",
    "dgram",
    "diagnostics_channel",
    "dns",
    "domain",
    "events",
    "fs",
    "fs/promises",
    "http",
    "http2",
    "https",
    "inspector",
    "module",
    "net",
    "os",
    "path",
    "perf_hooks",
    "process",
    "punycode",
    "querystring",
    "readline",
    "repl",
    "stream",
    "string_decoder",
    "sys",
    "timers",
    "tls",
    "trace_events",
    "tty",
    "url",
    "util",
    "v8",
    "vm",
    "wasi",
    "worker_threads",
    "zlib",
  ])
});
