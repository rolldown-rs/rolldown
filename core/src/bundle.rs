use crate::{graph};

// #[derive(Debug, Error)]
// pub enum BundleError {
//   #[error("{0}")]
//   GraphError(crate::graph::GraphError),
//   #[error("{0}")]
//   IoError(io::Error),
//   #[error("No Module found")]
//   NoModule,
// }

// impl From<io::Error> for BundleError {
//   fn from(err: io::Error) -> Self {
//     Self::IoError(err)
//   }
// }

// impl From<graph::GraphError> for BundleError {
//   fn from(err: graph::GraphError) -> Self {
//     Self::GraphError(err)
//   }
// }

#[derive(Clone)]
#[non_exhaustive]
pub struct Bundle {
  pub graph: graph::Graph,
}

impl Bundle {
  pub fn new(entry: &str) -> Self {
    Self {
      graph: graph::Graph::new(entry),
    }
  }

  // pub fn generate<W: Write>(
  //   self,
  //   w: W,
  //   sm: Option<&mut Vec<(BytePos, LineCol)>>,
  // ) -> Result<(), BundleError> {
  //   let statements = self.graph.build();
  //   statements
  //     .iter()
  //     .filter_map(|s| {
  //       self
  //         .graph
  //         .get_module(&s.module_id)
  //         .into_mod()
  //         .map(|m| (s, m))
  //     })
  //     .for_each(|(s, module)| {
  //       if s.is_export_declaration {
  //         analyse::fold_export_decl_to_decl(&mut s.node.write(), module);
  //       }
  //     });

  //   let mut emitter = swc_ecma_codegen::Emitter {
  //     cfg: swc_ecma_codegen::Config { minify: false },
  //     cm: graph::SOURCE_MAP.clone(),
  //     comments: None,
  //     wr: Box::new(JsWriter::with_target(
  //       graph::SOURCE_MAP.clone(),
  //       "\n",
  //       w,
  //       sm,
  //       EsVersion::latest(),
  //     )),
  //   };
  //   for stmt in statements {
  //     stmt.node.read().emit_with(&mut emitter)?;
  //   }
  //   Ok(())
  // }
}
