use std::sync::Arc;

use rolldown_core::{Bundle, Graph, NormalizedInputOptions, NormalizedOutputOptions, OutputChunk};

pub struct Rolldown {
    graph: Graph,
}

impl Rolldown {
    pub async fn build(&mut self) -> anyhow::Result<()> {
        self.graph.build().await?;

        tracing::trace!("graph {:#?}", self.graph);
        Ok(())
    }

    pub async fn write(
        &mut self,
        output_options: NormalizedOutputOptions,
    ) -> anyhow::Result<RolldownOutput> {
        self.graph.build().await?;
        // Bundle::new(self.graph.options.clone(), Default::default(), &mut self.graph).old_should_not_be_used_generate()?;
        let output_chunks = Bundle::new(
            self.graph.options.clone(),
            Default::default(),
            &mut self.graph,
        )
        .generate()?;

        std::fs::create_dir_all(format!("{}/dist", self.graph.options.root.as_str(),)).unwrap();
        output_chunks.iter().for_each(|chunk| {
            std::fs::write(
                format!(
                    "{}/dist/{}",
                    self.graph.options.root.as_str(),
                    chunk.filename
                ),
                &chunk.code,
            )
            .unwrap();
        });
        tracing::trace!("graph {:#?}", self.graph);
        Ok(RolldownOutput {
            output: output_chunks,
        })
    }

    pub async fn generate(
        &mut self,
        output_options: NormalizedOutputOptions,
    ) -> anyhow::Result<RolldownOutput> {
        self.graph.build().await?;
        let output_chunks = Bundle::new(
            self.graph.options.clone(),
            Default::default(),
            &mut self.graph,
        )
        .generate()?;

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
