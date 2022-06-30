use std::fmt::Debug;

use crate::{
  JobContext, LoadArgs, PluginContext, ResolveArgs, ResolvedId,
};

use anyhow::Result;
pub type PluginBuildStartHookOutput = Result<()>;
pub type PluginBuildEndHookOutput = Result<()>;
pub type PluginLoadHookOutput = Result<Option<String>>;
pub type PluginResolveHookOutput = Result<Option<ResolvedId>>;
// pub type PluginTransformAstHookOutput = Result<ast::Module>;
// pub type PluginParseOutput = Result<RspackAst>;
// pub type PluginGenerateOutput = Result<String>;
// pub type PluginTransformHookOutput = Result<TransformResult>;
// pub type PluginTapGeneratedChunkHookOutput = Result<()>;
// pub type PluginRenderChunkHookOutput = Result<OutputChunk>;

#[async_trait::async_trait]
pub trait Plugin: Debug + Send + Sync {
  async fn build_start(&self) -> PluginBuildStartHookOutput {
    Ok(())
  }

  async fn build_end(&self) -> PluginBuildEndHookOutput {
    Ok(())
  }

  async fn resolve(
    &self,
    _ctx: PluginContext<&mut JobContext>,
    _agrs: ResolveArgs<'_>,
  ) -> PluginResolveHookOutput {
    Ok(None)
  }

  async fn load(
    &self,
    _ctx: PluginContext<&mut JobContext>,
    _args: LoadArgs<'_>,
  ) -> PluginLoadHookOutput {
    Ok(None)
  }
}

#[derive(Debug)]
pub enum AssetFilename {
  Static(String),
  Templace(String),
}

#[derive(Debug)]
pub struct Asset {
  rendered: String,
  filename: AssetFilename,
  // pathOptions÷: PathData;
  // info?: AssetInfo;
  // pub identifier: String,
  // hash?: string;
  // auxiliary?: boolean;
}

impl Asset {
  pub fn new(rendered: String, filename: AssetFilename) -> Self {
    Self { rendered, filename }
  }

  pub fn source(&self) -> &str {
    self.rendered.as_str()
  }
}

impl Asset {
  pub fn final_filename(&self) -> String {
    match &self.filename {
      AssetFilename::Static(name) => name.clone(),
      AssetFilename::Templace(_) => todo!("Templace"),
    }
  }
}
