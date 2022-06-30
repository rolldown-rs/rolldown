use std::sync::Arc;

use crate::{Chunk, Graph, NormalizedOutputOptions, OutputChunk, NormalizedInputOptions};

#[derive(Debug)]
pub struct Bundle<'a> {
    pub input_options: Arc<NormalizedInputOptions>,
    pub options: NormalizedOutputOptions,
    pub graph: &'a mut Graph,
}

impl<'a> Bundle<'a> {
    pub fn new(input_options: Arc<NormalizedInputOptions>, options: NormalizedOutputOptions, graph: &'a mut Graph) -> Self {
        Self { input_options, options, graph }
    }

    pub fn generate(&self) -> anyhow::Result<()> {
        let chunks = self.generate_chunks()?;
        std::fs::create_dir_all(format!("{}/dist", self.input_options.root.as_str())).unwrap();
        chunks.iter().for_each(|chunk| {
            let code = chunk.render(&self.graph, &self.input_options);
            std::fs::write(format!("{}/dist/{}.js", self.input_options.root.as_str(), chunk.id), code).unwrap();
            
        });
        Ok(())
    }

    pub fn generate_output_chunks(&self) -> anyhow::Result<Vec<OutputChunk>> {
        let chunks = self.generate_chunks()?;
        Ok(chunks
            .iter()
            .map(|chunk| {
                let code = chunk.render(&self.graph, &self.input_options);
                // std::fs::write(format!("./dist/{}.js", chunk.id), code).unwrap();
                OutputChunk {
                    code,
                    filename: format!("{}.js", &chunk.id),
                }
            })
            .collect())
    }

    fn generate_chunks(&self) -> anyhow::Result<Vec<Chunk>> {
        let mut chunk = Chunk::new("main".into(), self.graph.resolved_entries[0].clone());
        self.graph
            .module_by_id
            .values()
            .map(|module| &module.id)
            .cloned()
            .for_each(|id| {
                chunk.module_ids.insert(id);
            });

        Ok(vec![chunk])
    }
}
