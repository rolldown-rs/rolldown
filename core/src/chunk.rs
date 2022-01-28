use dashmap::DashSet;
use std::{
  cmp::Ordering,
  collections::{HashMap, HashSet},
  sync::{Arc, Mutex},
};

use crate::{compiler, module::Module, renamer::Renamer, symbol_box::SymbolBox};

use rayon::prelude::*;
use swc_atoms::JsWord;
use swc_common::{
  comments::{Comment, CommentKind, Comments, SingleThreadedComments},
  Mark, SyntaxContext, DUMMY_SP,
};
use swc_ecma_ast::{EmptyStmt, EsVersion, ModuleItem, Stmt};
use swc_ecma_codegen::text_writer::JsWriter;
use swc_ecma_visit::VisitMutWith;

pub struct Chunk {
  pub order_modules: Vec<String>,
  pub symbol_box: Arc<Mutex<SymbolBox>>,
}

impl Chunk {
  pub fn new(order_modules: Vec<String>, symbol_box: Arc<Mutex<SymbolBox>>) -> Self {
    Self {
      order_modules,
      symbol_box,
    }
  }

  pub fn deconflict(&mut self, modules: &mut HashMap<String, Module>) {
    let mut used_names = HashSet::new();
    let mut mark_to_name = HashMap::new();
    let mut entry_first_modules = modules.values().collect::<Vec<_>>();
    entry_first_modules.sort_by(|a, b| {
      if a.is_user_defined_entry_point && !b.is_user_defined_entry_point {
        Ordering::Less
      } else if b.is_user_defined_entry_point && !a.is_user_defined_entry_point {
        Ordering::Greater
      } else {
        Ordering::Equal
      }
    });
    entry_first_modules.into_iter().for_each(|module| {
      module.declared_symbols.iter().for_each(|(name, mark)| {
        let root_mark = self.symbol_box.lock().unwrap().find_root(*mark);
        if mark_to_name.contains_key(&root_mark) {
        } else {
          let original_name = name.to_string();
          let mut name = name.to_string();
          let mut count = 0;
          while used_names.contains(&name) {
            name = format!("{}${}", original_name, count);
            count += 1;
          }
          mark_to_name.insert(root_mark, name.clone());
          used_names.insert(name);
        }
      });
    });

    modules.par_iter_mut().for_each(|(_, module)| {
      let mut renamer = Renamer {
        mark_to_names: &mark_to_name,
        symbox_box: self.symbol_box.clone(),
      };
      module.ast.visit_mut_with(&mut renamer);
    });

    log::debug!("mark_to_name {:#?}", mark_to_name);
  }

  pub fn render(&mut self, modules: &mut HashMap<String, Module>) -> String {
    // let modules = modules.par_iter_mut().map(|(key, module)| (key.clone(), module))
    modules.par_iter_mut().for_each(|(_key, module)| {
      module.trim_exports();
      if module.is_user_defined_entry_point {
        module.generate_exports();
      }
    });

    self.deconflict(modules);

    let mut output = Vec::new();
    let comments = SingleThreadedComments::default();

    // TODO: There's an problem in SWC, so we had to do following. See https://github.com/swc-project/swc/issues/3354.
    self.order_modules.iter().for_each(|id| {
      // filter external modules
      if let Some(module) = modules.get_mut(id) {
        module.ast.body.insert(
          0,
          ModuleItem::Stmt(Stmt::Empty(EmptyStmt {
            span: swc_common::Span {
              lo: module.ast.span.lo,
              hi: module.ast.span.hi,
              ..Default::default()
            },
          })),
        );
        comments.add_leading(
          module.ast.span.lo,
          Comment {
            kind: CommentKind::Line,
            span: DUMMY_SP,
            text: format!(" {}", module.id),
          },
        );
      }
    });

    // compiler::COMPILER.cm

    let mut emitter = swc_ecma_codegen::Emitter {
      cfg: swc_ecma_codegen::Config { minify: false },
      cm: compiler::COMPILER.cm.clone(),
      comments: Some(&comments),
      wr: Box::new(JsWriter::with_target(
        compiler::COMPILER.cm.clone(),
        "\n",
        &mut output,
        None,
        EsVersion::latest(),
      )),
    };

    self.order_modules.iter().for_each(|idx| {
      if let Some(module) = modules.get(idx) {
        emitter.emit_module(&module.ast).unwrap();
      }
    });

    String::from_utf8(output).unwrap()
  }
}
