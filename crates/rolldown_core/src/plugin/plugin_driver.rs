use std::{collections::HashMap, sync::Arc};

use nodejs_resolver::Resolver;
use tracing::instrument;

use crate::{
  Asset, JobContext, LoadArgs, Plugin, PluginContext,
  PluginResolveHookOutput, ResolveArgs, NormalizedInputOptions, PluginLoadHookOutput,
};

#[derive(Debug)]
pub struct PluginDriver {
  pub(crate) options: Arc<NormalizedInputOptions>,
  pub plugins: Vec<Box<dyn Plugin>>,
  // pub resolver: Arc<Resolver>,
}

impl PluginDriver {
  pub fn new(
    options: Arc<NormalizedInputOptions>,
    plugins: Vec<Box<dyn Plugin>>,
    // resolver: Arc<Resolver>,
  ) -> Self {
    Self {
      options,
      plugins,
      // resolver,
    }
  }

  pub async fn resolve(
    &self,
    args: ResolveArgs<'_>,
    job_ctx: &mut JobContext,
  ) -> PluginResolveHookOutput {
    for plugin in &self.plugins {
      let output = plugin
        .resolve(PluginContext::with_context(job_ctx), args.clone())
        .await?;
      if output.is_some() {
        return Ok(output);
      }
    }
    Ok(None)
  }

  pub async fn load(
    &self,
    args: LoadArgs<'_>,
    job_ctx: &mut JobContext,
  ) -> PluginLoadHookOutput {
    for plugin in &self.plugins {
      let output = plugin
        .load(PluginContext::with_context(job_ctx), args.clone())
        .await?;
      if output.is_some() {
        return Ok(output);
      }
    }
    Ok(None)
  }
}
