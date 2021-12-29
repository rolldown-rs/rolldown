use std::collections::HashMap;

use swc_atoms::JsWord;
use swc_common::SyntaxContext;
use swc_ecma_ast::{Expr, Ident, KeyValueProp, ObjectLit, Prop, PropName, PropOrSpread};
use swc_ecma_visit::{VisitMut, VisitMutWith};

pub struct Renamer<'me> {
  pub ctxt_mapping: &'me HashMap<JsWord, SyntaxContext>,
  pub mapping: &'me HashMap<JsWord, JsWord>,
}

impl<'me> VisitMut for Renamer<'me> {
  fn visit_mut_ident(&mut self, node: &mut Ident) {
    if let Some(ctxt) = self.ctxt_mapping.get(&node.sym) {
      if &node.span.ctxt == ctxt {
        if let Some(replacement) = self.mapping.get(&node.sym).map(|s| s.clone()) {
          node.sym = replacement
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
