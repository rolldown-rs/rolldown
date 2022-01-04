use std::collections::HashMap;

use ena::unify::InPlaceUnificationTable;
use swc_atoms::JsWord;
use swc_common::SyntaxContext;
use swc_ecma_ast::{Expr, Ident, ImportDecl, KeyValueProp, ObjectLit, Prop, PropName, PropOrSpread};
use swc_ecma_visit::{VisitMut, VisitMutWith};

use crate::chunk::Ctxt;

pub struct Renamer<'me> {
  pub ctxt_mapping: &'me HashMap<JsWord, SyntaxContext>,
  pub mapping: &'me HashMap<JsWord, JsWord>,
  pub symbol_uf: &'me mut InPlaceUnificationTable<Ctxt>,
  pub ctxt_to_name: &'me HashMap<SyntaxContext, JsWord>
}

impl<'me> VisitMut for Renamer<'me> {
  fn visit_mut_import_decl(&mut self, node: &mut ImportDecl) {
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
    let ctxt: Ctxt = node.span.ctxt.clone().into();
    let root = self.symbol_uf.find(ctxt);
    if let Some(replacement) = self.ctxt_to_name.get(&root.0) {
      node.sym = replacement.clone()
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
