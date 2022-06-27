#[derive(Debug, Default)]
pub struct PluginContext<T = ()> {
  pub context: T,
}

impl PluginContext {
  pub fn new() -> Self {
    Self::with_context(())
  }
}

impl<T> PluginContext<T> {
  pub fn with_context(context: T) -> Self {
    Self { context }
  }
}
