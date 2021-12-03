pub mod scope;

pub mod helper {
  use swc_ecma_ast::Pat;
  pub fn collect_names_of_pat(pat: &Pat) -> Vec<String> {
    match pat {
      // export const a = 1;
      Pat::Ident(pat) => vec![pat.id.sym.to_string()],
      // export const [a] = [1]
      Pat::Array(pat) => pat
        .elems
        .iter()
        .flat_map(|pat| pat.as_ref().map_or(vec![], collect_names_of_pat))
        .collect(),
      // TODO: export const { a } = { a: 1 }
      // Pat::Object()
      _ => vec![],
    }
  }
}
