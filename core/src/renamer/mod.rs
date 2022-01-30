use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
};

use swc_common::{Mark, SyntaxContext};
use swc_ecma_ast::{
  ExportNamedSpecifier, Expr, Ident, ImportDecl, KeyValueProp, ObjectLit, Prop,
  PropName, PropOrSpread, ModuleExportName,
};
use swc_ecma_visit::{VisitMut, VisitMutWith};

use crate::{ext::SyntaxContextExt, symbol_box::SymbolBox};

#[derive(Debug)]
pub struct Renamer<'me> {
  pub mark_to_names: &'me HashMap<Mark, String>,
  pub symbox_box: Arc<Mutex<SymbolBox>>,
}

impl<'me> VisitMut for Renamer<'me> {
  fn visit_mut_import_decl(&mut self, _node: &mut ImportDecl) {
    // We won't remove import statement which import external module. So we need to consider following situation
    // ```a.js
    // import { useState } from 'react'
    // console.log(useState)
    // ```
    // ```b.js
    // const useState = () => {}
    // useState()
    // ```
    // ```a+b.js
    // import { useState as useState$1 } from 'react'
    // console.log(useState$1)
    // const useState = () => {}
    // useState()
    // ```
    // TODO:
  }

  fn visit_mut_ident(&mut self, node: &mut Ident) {
    let mark = node.span.ctxt.as_mark();
    let root_mark = self.symbox_box.lock().unwrap().find_root(mark);
    if let Some(name) = self.mark_to_names.get(&root_mark) {
      node.sym = name.to_string().into()
    }
  }

  fn visit_mut_export_named_specifier(&mut self, node: &mut ExportNamedSpecifier) {
    node.visit_mut_children_with(self);
    if let Some(ModuleExportName::Ident(expr)) = &node.exported {
      if let ModuleExportName::Ident(orig) = &node.orig {
        if expr.sym == orig.sym {
          node.exported = None
        }
      }
    }
  }

  fn visit_mut_object_lit(&mut self, node: &mut ObjectLit) {
    node
      .props
      .iter_mut()
      .for_each(|prop_or_spread| match prop_or_spread {
        PropOrSpread::Prop(prop) => {
          if prop.is_shorthand() {
            if let Prop::Shorthand(ident) = prop.as_mut() {
              let mut key = ident.clone();
              key.span.ctxt = SyntaxContext::empty();
              let replacement = Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(key),
                value: Box::new(Expr::Ident(ident.clone())),
              }));
              *prop = replacement;
            }
          }
        }
        _ => {}
      });
    node.visit_mut_children_with(self);
  }
}
