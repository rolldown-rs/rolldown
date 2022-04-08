use crate::utils::lcp_of_array;
use dashmap::DashSet;
use smol_str::SmolStr;
use std::{
  collections::{HashMap, HashSet},
  path::Path,
  sync::{Arc, Mutex},
};

use crate::{
  compiler::SOURCE_MAP,
  module::Module,
  renamer::Renamer,
  structs::{OutputChunk, RenderedChunk},
  symbol_box::SymbolBox,
  types::NormalizedOutputOptions,
  utils::lcp,
};

use rayon::prelude::*;

use swc_common::comments::{Comment, Comments, SingleThreadedComments};
use swc_ecma_ast::EsVersion;
use swc_ecma_codegen::text_writer::JsWriter;
use swc_ecma_visit::VisitMutWith;

pub struct Chunk {
  pub id: SmolStr,
  pub order_modules: Vec<SmolStr>,
  pub symbol_box: Arc<Mutex<SymbolBox>>,
  pub entries: DashSet<SmolStr>,
}

impl Chunk {
  pub fn new(
    order_modules: Vec<SmolStr>,
    symbol_box: Arc<Mutex<SymbolBox>>,
    entries: DashSet<SmolStr>,
  ) -> Self {
    Self {
      id: Default::default(),
      order_modules,
      symbol_box,
      entries,
    }
  }

  pub fn de_conflict(&mut self, modules: &mut HashMap<SmolStr, Box<Module>>) {
    let mut used_names = HashSet::new();
    let mut mark_to_name = HashMap::new();

    // De-conflict from the entry module to keep namings as simple as possible
    self
      .order_modules
      .iter()
      .map(|id| modules.get(id).unwrap())
      .rev()
      .for_each(|module| {
        module.declared_symbols.iter().for_each(|(name, mark)| {
          let root_mark = self.symbol_box.lock().unwrap().find_root(*mark);
          if let std::collections::hash_map::Entry::Vacant(e) = mark_to_name.entry(root_mark) {
            let original_name = name.to_string();
            let mut name = name.to_string();
            let mut count = 0;
            while used_names.contains(&name) {
              name = format!("{}${}", original_name, count);
              count += 1;
            }
            e.insert(name.clone());
            used_names.insert(name);
          } else {
          }
        });
      });

    modules.par_iter_mut().for_each(|(_, module)| {
      module.statements.iter_mut().for_each(|stmt| {
        let mut renamer = Renamer {
          mark_to_name: &mark_to_name,
          symbol_box: self.symbol_box.clone(),
        };
        stmt.node.visit_mut_with(&mut renamer);
      });
    });

    log::debug!("mark_to_name {:#?}", mark_to_name);
  }

  pub fn render(
    &mut self,
    options: &NormalizedOutputOptions,
    modules: &mut HashMap<SmolStr, Box<Module>>,
  ) -> RenderedChunk {
    assert!(!self.id.is_empty());
    modules.par_iter_mut().for_each(|(_key, module)| {
      module.trim_exports();
      if module.is_user_defined_entry_point {
        module.generate_exports();
      }
    });

    self.de_conflict(modules);

    let common_prefix = lcp_of_array(&self.order_modules);
    let common_prefix_len = if let Ok(p) = std::env::current_dir().map(|p| p.display().to_string())
    {
      lcp(&p, &common_prefix).len()
    } else {
      common_prefix.len()
    };
    let mut output = Vec::new();
    let comments = SingleThreadedComments::default();

    self.order_modules.iter().for_each(|idx| {
      if let Some(module) = modules.get_mut(idx) {
        let mut text = String::with_capacity(module.id.len() + 1);
        text.push(' ');
        text.push_str(&module.id[common_prefix_len..]);
        comments.add_leading(
          module.module_span.lo,
          Comment {
            kind: swc_common::comments::CommentKind::Line,
            span: module.module_span,
            text,
          },
        )
      }
    });

    let mut emitter = swc_ecma_codegen::Emitter {
      cfg: swc_ecma_codegen::Config {
        minify: options.minify,
      },
      cm: SOURCE_MAP.clone(),
      comments: Some(&comments),
      wr: Box::new(JsWriter::with_target(
        SOURCE_MAP.clone(),
        "\n",
        &mut output,
        None,
        EsVersion::latest(),
      )),
    };

    self.order_modules.iter().for_each(|idx| {
      if let Some(module) = modules.get(idx) {
        module.render(&mut emitter);
      }
    });

    RenderedChunk {
      code: String::from_utf8(output).unwrap(),
      file_name: self.id.clone().into(),
    }
  }

  pub fn get_chunk_info_with_file_names(&self) -> OutputChunk {
    OutputChunk {
      code: "".to_string(),
      file_name: self.id.clone().into(),
    }
  }

  #[inline]
  pub fn get_fallback_chunk_name(&self) -> &str {
    get_alias_name(self.order_modules.last().unwrap())
  }

  #[inline]
  pub fn get_chunk_name(&self) -> &str {
    self.get_fallback_chunk_name()
  }

  pub fn generate_id(&self, options: &NormalizedOutputOptions) -> SmolStr {
    let pattern = &options.entry_file_names;
    pattern.replace("[name]", self.get_chunk_name()).into()
  }
}

#[inline]
fn get_alias_name(id: &str) -> &str {
  let p = Path::new(id);
  // +1 to include `.`
  let ext_len = p.extension().map_or(0, |s| s.to_string_lossy().len() + 1);
  let file_name = p.file_name().unwrap().to_str().unwrap();
  &file_name[0..file_name.len() - ext_len]
}
