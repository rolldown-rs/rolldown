use std::sync::Arc;

use rolldown_core::{Bundle, Graph, NormalizedInputOptions, OutputChunk};

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
        Bundle::new(self.graph.options.clone(), Default::default(), &mut self.graph).generate()?;

        tracing::trace!("graph {:#?}", self.graph);
        Ok(())
    }

    pub async fn generate(&mut self) -> anyhow::Result<RolldownOutput> {
        self.graph.build().await?;
        let output_chunks =
            Bundle::new(self.graph.options.clone(), Default::default(), &mut self.graph).generate_output_chunks()?;

        tracing::trace!("graph {:#?}", self.graph);
        Ok(RolldownOutput {
            output: output_chunks,
        })
    }
}

pub fn rolldown(options: NormalizedInputOptions) -> Rolldown {
    let graph = Graph::new(Arc::new(options), vec![]);
    Rolldown { graph }
}

pub struct RolldownOutput {
   pub output: Vec<OutputChunk>,
}

impl RolldownOutput {
  fn chunk_by_id(&self, id: &str) -> Option<&OutputChunk> {
    self.output.iter().find(|chunk| chunk.filename == id)
  }
}
