




use crate::{types::{shared, NormalizedInputOptions, ResolveIdResult, Shared}};

pub struct PluginDriver {
  pub options: Shared<NormalizedInputOptions>,
  pub plugins: Vec<Box<dyn Plugin>>,
}

impl PluginDriver {
  pub fn new(options: Shared<NormalizedInputOptions>) -> Shared<Self> {
    shared(Self {
      options,
      plugins: vec![],
    })
  }
}

// Align to https://rollupjs.org/guide/en/#build-hooks

impl PluginDriver {
  pub fn options(&self) {}

  pub fn build_start(&self, options: &NormalizedInputOptions) {
    // TODO: should be parallel
    self.plugins.iter().for_each(|plugin| plugin.build_start(options))
  }

  #[inline]
  pub fn resolve_id(&self, source: &str, importer: Option<&str>) -> ResolveIdResult {
    let result = self
      .plugins
      .iter()
      .map(|plugin| plugin.resolve_id(source, importer))
      .take_while(|result| result.is_some())
      .map(|r| r.unwrap())
      .next();

    result
  }

  #[inline]
  pub fn load(&self, id: &str) -> Option<String> {
    let result = self
      .plugins
      .iter()
      .map(|plugin| plugin.load(id))
      .take_while(|result| result.is_some())
      .map(|r| r.unwrap())
      .next();

    result
  }

  pub fn transform(&self, _code: String, _id: &str) -> Option<String> {
    None
  }

  pub fn module_parsed(&self) {
    // TODO: should be parallel
    self
      .plugins
      .iter()
      .for_each(|plugin| plugin.module_parsed())
  }

  pub fn resolve_dynamic_import(&self, specifier: &str, importer: &str) -> ResolveIdResult {
    let result = self
      .plugins
      .iter()
      .map(|plugin| plugin.resolve_dynamic_import(specifier, importer))
      .take_while(|result| result.is_some())
      .map(|r| r.unwrap())
      .next();

    result
  }

  pub fn build_end(&self) {
    // TODO: should be parallel
    self.plugins.iter().for_each(|plugin| plugin.build_end(None))
  }
}

pub trait Plugin {
  // Align to https://rollupjs.org/guide/en/#build-hooks

  fn get_name(&self) -> &'static str;

  #[inline]
  fn options(&self, _options: &NormalizedInputOptions) -> Option<NormalizedInputOptions> {
    // async, sequential
    None
  }

  #[inline]
  fn build_start(&self, _options: &NormalizedInputOptions) {
    //  async, parallel
  }

  #[inline]
  fn resolve_id(&self, _source: &str, _importer: Option<&str>) -> ResolveIdResult {
    //  async, first
    None
  }

  #[inline]
  fn load(&self, _id: &str) -> Option<String> {
    // async, first
    // TODO: call hook load of plugins
    None
  }

  #[inline]
  fn transform(&self, _code: String, _id: &str) -> Option<String> {
    None
  }

  #[inline]
  fn module_parsed(&self) {
    // async, parallel
  }

  #[inline]
  fn resolve_dynamic_import(&self, _specifier: &str, _importer: &str) -> ResolveIdResult {
    //  async, first
    None
  }

  #[inline]
  fn build_end(&self, _err: Option<Box<dyn std::error::Error>>) {
    // async, parallel
  }
}
