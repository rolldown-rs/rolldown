pub mod log;
use std::path::{Component, Path};

use sugar_path::PathSugar;

mod hooks;
pub use hooks::*;

pub fn uri_to_chunk_name(root: &str, uri: &str) -> String {
  let path = Path::new(uri);
  let mut relatived = Path::new(path).relative(root);
  let ext = relatived
    .extension()
    .and_then(|ext| ext.to_str())
    .unwrap_or("")
    .to_string();
  relatived.set_extension("");
  let mut name = relatived
    .components()
    .filter(|com| matches!(com, Component::Normal(_)))
    .filter_map(|seg| seg.as_os_str().to_str())
    .intersperse("_")
    .fold(String::new(), |mut acc, seg| {
      acc.push_str(seg);
      acc
    });
  name.push('_');
  name.push_str(&ext);
  name
}

pub fn parse_to_url(uri: &str) -> url::Url {
  if !uri.contains(':') {
    url::Url::parse(&format!("specifier:{}", uri)).unwrap()
  } else {
    url::Url::parse(uri).unwrap()
  }
}


use once_cell::sync::Lazy;
use std::sync::Arc;
use swc::{config::IsModule, Compiler as SwcCompiler};
use swc_common::{FileName, FilePathMapping, SourceMap};
use swc_ecma_parser::Syntax;
use swc_ecma_parser::{EsConfig, TsConfig};
use tracing::instrument;

static SWC_COMPILER: Lazy<Arc<SwcCompiler>> = Lazy::new(|| {
  let cm = Arc::new(SourceMap::new(FilePathMapping::empty()));

  Arc::new(SwcCompiler::new(cm))
});

pub fn get_swc_compiler() -> Arc<SwcCompiler> {
  SWC_COMPILER.clone()
}

#[instrument(skip_all)]
pub fn parse_file(
  source_code: String,
  filename: &str,
  // source_type: &SourceType,
) -> swc_ecma_ast::Program {
  let syntax = syntax_by_source_type(filename, "js");
  let compiler = get_swc_compiler();
  let fm = compiler
    .cm
    .new_source_file(FileName::Custom(filename.to_string()), source_code);
  swc::try_with_handler(compiler.cm.clone(), Default::default(), |handler| {
    compiler.parse_js(
      fm,
      handler,
      swc_ecma_ast::EsVersion::Es2022,
      syntax,
      // TODO: Is this correct to think the code is module by default?
      IsModule::Bool(true),
      None,
    )
  })
  .unwrap()
}

pub fn syntax_by_ext(ext: &str) -> Syntax {
  match ext == "ts" || ext == "tsx" {
    true => Syntax::Typescript(TsConfig {
      decorators: false,
      tsx: ext == "tsx",
      ..Default::default()
    }),
    false => Syntax::Es(EsConfig {
      private_in_object: true,
      import_assertions: true,
      jsx: ext == "jsx",
      export_default_from: true,
      decorators_before_export: true,
      decorators: true,
      fn_bind: true,
      allow_super_outside_method: true,
    }),
  }
}

pub fn syntax_by_source_type(filename: &str, ext: &str) -> Syntax {
  match ext {
    "js" | "jsx" => Syntax::Es(EsConfig {
      private_in_object: true,
      import_assertions: true,
      jsx: matches!(ext, "jsx"),
      export_default_from: true,
      decorators_before_export: true,
      decorators: true,
      fn_bind: true,
      allow_super_outside_method: true,
    }),
    "ts" | "tsx" => Syntax::Typescript(TsConfig {
      decorators: false,
      tsx: matches!(ext, "tsx"),
      ..Default::default()
    }),
    _ => {
      let ext = Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("js");
      syntax_by_ext(ext)
    }
  }
}
