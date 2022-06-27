use std::sync::Arc;

use rolldown_core::{NormalizedInputOptions, Graph, Bundle};



pub struct Rolldown {
  graph: Graph,
}

impl Rolldown {

  pub async fn build(&mut self) -> anyhow::Result<()> {
    self.graph.build().await?;

    tracing::trace!("graph {:#?}", self.graph);
    Ok(())
  }

  pub async fn write(&mut self) -> anyhow::Result<()> {
    self.graph.build().await?;
    Bundle::new(Default::default(), &mut self.graph).generate()?;

    tracing::trace!("graph {:#?}", self.graph);
    Ok(())
  }
}

pub fn rolldown(options: NormalizedInputOptions) -> Rolldown {

  let graph = Graph::new(Arc::new(options), vec![]);
  Rolldown { graph }
}