use crate::{Chunk, Graph, NormalizedOutputOptions};

#[derive(Debug)]
pub struct Bundle<'a> {
    pub options: NormalizedOutputOptions,
    pub graph: &'a mut Graph,
}

impl<'a> Bundle<'a> {
    pub fn new(options: NormalizedOutputOptions, graph: &'a mut Graph) -> Self {
        Self { options, graph }
    }

    pub fn generate(&self) -> anyhow::Result<()> {
        let chunks = self.generate_chunks()?;
        chunks.iter().for_each(|chunk| {
            let code = chunk.render(&self.graph);
            std::fs::write(format!("./dist/{}.js", chunk.id), code).unwrap();
        });
        Ok(())
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
