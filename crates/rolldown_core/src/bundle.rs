use std::sync::Arc;

use hashbrown::HashMap;

use crate::{
    Chunk, Graph, NormalizedInputOptions, NormalizedOutputOptions, OutputChunk, PrepareContext,
};
use rayon::prelude::*;

#[derive(Debug)]
pub struct Bundle<'a> {
    pub input_options: Arc<NormalizedInputOptions>,
    pub options: NormalizedOutputOptions,
    pub graph: &'a mut Graph,
}

impl<'a> Bundle<'a> {
    pub fn new(
        input_options: Arc<NormalizedInputOptions>,
        options: NormalizedOutputOptions,
        graph: &'a mut Graph,
    ) -> Self {
        Self {
            input_options,
            options,
            graph,
        }
    }

    pub fn generate(&mut self) -> anyhow::Result<Vec<OutputChunk>> {
        let chunks = self.generate_chunks()?;
        {
            chunks
                .iter()
                .map(|chunk| {
                    let prepare_context = PrepareContext {
                      uf: &self.graph.uf,
                        unresolved_mark: self.graph.unresolved_mark,
                        modules: chunk
                            .module_ids
                            .iter()
                            .map(|module_id| {
                                self.graph.module_by_id.remove_entry(module_id).unwrap()
                            })
                            .collect::<HashMap<_, _, _>>(),
                    };

                    (chunk, prepare_context)
                })
                .par_bridge()
                .into_par_iter()
                .map(|(chunk, mut prepare_context)| {
                    chunk.prepare(&mut prepare_context);
                    prepare_context
                })
                .collect::<Vec<_>>()
                .into_iter()
                .for_each(|prepare_context| {
                    self.graph.module_by_id.extend(prepare_context.modules);
                });
        };
        Ok(chunks
            .par_iter()
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
