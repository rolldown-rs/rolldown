#[derive(Debug, Clone)]
pub struct ResolveArgs<'a> {
  pub importer: Option<&'a str>,
  pub specifier: &'a str,
}

#[derive(Debug, Clone)]
pub struct LoadArgs<'a> {
  pub id: &'a str,
}

pub struct TransformArgs {
  pub source: String,
}
