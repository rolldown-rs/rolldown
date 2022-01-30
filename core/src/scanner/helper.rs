use swc_atoms::JsWord;
use swc_ecma_ast::{Ident, ObjectPatProp, Pat};

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
