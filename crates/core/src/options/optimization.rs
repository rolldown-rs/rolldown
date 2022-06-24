use std::str::FromStr;

// use crate::BundleMode;

// #[derive(Debug, Clone)]
// pub struct OptimizationOptions {
//     pub chunk_id_algo: ChunkIdAlgo,
//     pub module_id_algo: ModuleIdAlgo,
//     pub remove_empty_chunks: bool,
// }

// impl Default for OptimizationOptions {
//     fn default() -> Self {
//         Self {
//             chunk_id_algo: ChunkIdAlgo::Named,
//             module_id_algo: ModuleIdAlgo::Named,
//             remove_empty_chunks: true,
//         }
//     }
// }

#[derive(Debug, Clone)]
pub enum ChunkIdAlgo {
  /// Readable ids for better debugging.
  Named,
  /// Numeric ids in order of usage.
  Numeric,
}

impl ChunkIdAlgo {
  pub fn is_named(&self) -> bool {
    matches!(self, ChunkIdAlgo::Named)
  }

  pub fn is_numeric(&self) -> bool {
    matches!(self, ChunkIdAlgo::Numeric)
  }
}

#[derive(Debug, Clone)]
pub enum ModuleIdAlgo {
  /// Readable ids for better debugging.
  Named,
  /// Numeric ids in order of usage.
  Numeric,
}

impl ModuleIdAlgo {
  pub fn is_named(&self) -> bool {
    matches!(self, Self::Named)
  }

  pub fn is_numeric(&self) -> bool {
    matches!(self, Self::Numeric)
  }
}

impl FromStr for ChunkIdAlgo {
  type Err = ();

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "named" => Result::Ok(Self::Named),
      _ => Err(()),
    }
  }
}

impl FromStr for ModuleIdAlgo {
  type Err = ();

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "named" => Result::Ok(Self::Named),
      _ => Err(()),
    }
  }
}

// impl From<BundleMode> for OptimizationOptions {
//     fn from(mode: BundleMode) -> Self {
//         Self {
//             chunk_id_algo: {
//                 if mode.is_prod() {
//                     ChunkIdAlgo::Numeric
//                 } else {
//                     ChunkIdAlgo::Named
//                 }
//             },
//             module_id_algo: {
//                 if mode.is_prod() {
//                     ModuleIdAlgo::Numeric
//                 } else {
//                     ModuleIdAlgo::Named
//                 }
//             },
//             remove_empty_chunks: !mode.is_none(),
//         }
//     }
// }
