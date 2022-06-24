#![feature(iter_intersperse)]

mod module;
pub use module::*;
use std::sync::Arc;

// use dashmap::DashSet;
// pub use module::*;
mod plugin;
pub use plugin::*;
mod resolving_module_job;
pub use resolving_module_job::*;
mod compiler;
pub use compiler::*;
use dashmap::DashSet;
mod options;
pub use options::*;
mod module_graph;
pub use module_graph::*;
mod chunk;
pub use chunk::*;
mod utils;
use swc_atoms::JsWord;
pub use utils::*;
mod chunk_graph;
pub use chunk_graph::*;
mod chunk_spliter;
pub use chunk_spliter::*;
mod stats;
pub use stats::*;
mod visitors;
pub use visitors::*;

pub(crate) type VisitedModuleIdentity = Arc<DashSet<JsWord>>;
