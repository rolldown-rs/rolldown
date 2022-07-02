pub mod log;
use std::path::{Component, Path};

use ast::{Pat, Ident, ObjectPatProp};
use sugar_path::PathSugar;

mod hooks;
pub use hooks::*;
mod side_effect;
pub use side_effect::*;
mod shake;
pub use shake::*;
mod name_helpers;
pub use name_helpers::*;

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
use swc_atoms::JsWord;
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


pub fn parse_file(
  source_code: String,
  filename: &str,
  // source_type: &SourceType,
) -> ast::Program {
  let syntax = syntax_by_source_type(filename, "js");
  let compiler = get_swc_compiler();
  let fm = compiler
    .cm
    .new_source_file(FileName::Custom(filename.to_string()), source_code);
  swc::try_with_handler(compiler.cm.clone(), Default::default(), |handler| {
    compiler.parse_js(
      fm,
      handler,
      ast::EsVersion::Es2022,
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
      allow_return_outside_function: true,
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
      allow_return_outside_function: true,
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


#[inline]
pub fn collect_ident_of_pat(pat: &Pat) -> Vec<&Ident> {
  match pat {
    // export const a = 1;
    Pat::Ident(pat) => vec![&pat.id],
    // export const [a] = [1]
    Pat::Array(pat) => pat
      .elems
      .iter()
      .flat_map(|pat| pat.as_ref().map_or(vec![], collect_ident_of_pat))
      .collect(),
    Pat::Object(pat) => pat
      .props
      .iter()
      .flat_map(|prop_pat| match prop_pat {
        ObjectPatProp::Assign(pat) => {
          vec![&pat.key]
        }
        ObjectPatProp::KeyValue(pat) => collect_ident_of_pat(pat.value.as_ref()),
        ObjectPatProp::Rest(pat) => collect_ident_of_pat(pat.arg.as_ref()),
      })
      .collect(),
    Pat::Assign(pat) => collect_ident_of_pat(pat.left.as_ref()),
    _ => vec![],
  }
}

pub fn collect_mut_ident_of_pat(pat: &mut Pat) -> Vec<&mut Ident> {
  match pat {
    // export const a = 1;
    Pat::Ident(pat) => vec![&mut pat.id],
    // export const [a] = [1]
    Pat::Array(pat) => pat
      .elems
      .iter_mut()
      .flat_map(|pat| pat.as_mut().map_or(vec![], collect_mut_ident_of_pat))
      .collect(),
    Pat::Object(pat) => pat
      .props
      .iter_mut()
      .flat_map(|prop_pat| match prop_pat {
        ObjectPatProp::Assign(pat) => {
          vec![&mut pat.key]
        }
        ObjectPatProp::KeyValue(pat) => collect_mut_ident_of_pat(pat.value.as_mut()),
        ObjectPatProp::Rest(pat) => collect_mut_ident_of_pat(pat.arg.as_mut()),
      })
      .collect(),
    Pat::Assign(pat) => collect_mut_ident_of_pat(pat.left.as_mut()),
    _ => vec![],
  }
}

#[inline]
pub fn collect_js_word_of_pat(pat: &Pat) -> Vec<JsWord> {
  collect_ident_of_pat(pat)
    .into_iter()
    .map(|id| id.sym.clone())
    .collect()
}

