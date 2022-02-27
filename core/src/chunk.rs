use dashmap::DashSet;
use smol_str::SmolStr;
use std::{
  collections::{HashMap, HashSet},
  fmt::format,
  path::Path,
  sync::{Arc, Mutex},
  time::Instant,
};

use crate::{
  compiler::SOURCE_MAP,
  module::Module,
  renamer::Renamer,
  structs::{OutputChunk, RenderedChunk},
  symbol_box::SymbolBox,
  types::NormalizedOutputOptions,
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

  pub fn deconflict(&mut self, modules: &mut HashMap<SmolStr, Module>) {
    let start = Instant::now();

    let mut used_names = HashSet::new();
    let mut mark_to_name = HashMap::new();

    // Deconflict from the entry module to keep namings as simple as possible
    let reverse_ordered_modules = self
      .order_modules
      .iter()
      .map(|id| modules.get(id).unwrap())
      .rev()
      .collect::<Vec<_>>();

    reverse_ordered_modules.into_iter().for_each(|module| {
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

    let rename_start = Instant::now();

    modules.par_iter_mut().for_each(|(_, module)| {
      module.statements.iter_mut().for_each(|stmt| {
        let mut renamer = Renamer {
          mark_to_names: &mark_to_name,
          symbox_box: self.symbol_box.clone(),
        };
        stmt.node.visit_mut_with(&mut renamer);
      });
    });

    println!(
      "Chunk#deconflict()-rename finished in {}",
      rename_start.elapsed().as_millis()
    );

    println!(
      "Chunk#deconflict() finished in {}",
      start.elapsed().as_millis()
    );

    log::debug!("mark_to_name {:#?}", mark_to_name);
  }

  pub fn render(
    &mut self,
    options: &NormalizedOutputOptions,
    modules: &mut HashMap<SmolStr, Module>,
  ) -> RenderedChunk {
    assert!(!self.id.is_empty());
    let prune_start = Instant::now();
    let render_chunk_start = Instant::now();
    modules.par_iter_mut().for_each(|(_key, module)| {
      module.trim_exports();
      if module.is_user_defined_entry_point {
        module.generate_exports();
      }
    });

    println!(
      "prune modules finished in {}",
      prune_start.elapsed().as_millis()
    );

    self.deconflict(modules);

    let emit_start = Instant::now();

    let mut output = Vec::new();
    let comments = SingleThreadedComments::default();

    self.order_modules.iter().for_each(|idx| {
      if let Some(module) = modules.get_mut(idx) {
        comments.add_leading(
          module.module_span.lo,
          Comment {
            kind: swc_common::comments::CommentKind::Line,
            span: module.module_span.clone(),
            text: format!(" {}", &module.id),
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

    let rendered_chunk = RenderedChunk {
      code: String::from_utf8(output).unwrap(),
      file_name: self.id.clone().into(),
    };

    println!(
      "Chunk#render()-emit finished in {}",
      emit_start.elapsed().as_millis()
    );

    println!(
      "Chunk#render() finished in {}",
      render_chunk_start.elapsed().as_millis()
    );

    rendered_chunk
  }

  pub fn get_chunk_info_with_file_names(&self) -> OutputChunk {
    OutputChunk {
      code: "".to_string(),
      file_name: self.id.clone().into(),
    }
  }

  #[inline]
  pub fn get_fallback_chunk_name(&self) -> &str {
    // if (this.manualChunkAlias) {
    // 	return this.manualChunkAlias;
    // }
    // if (this.dynamicName) {
    // 	return this.dynamicName;
    // }
    // if (this.fileName) {
    // 	return getAliasName(this.fileName);
    // }
    get_alias_name(self.order_modules.last().unwrap())
    // return getAliasName(this.orderedModules[this.orderedModules.length - 1].id);
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
