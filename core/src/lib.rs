#![deny(clippy::all)]

mod ast;
mod bundle;
mod chunk;
mod external_module;
mod graph;
mod module;
mod module_loader;
mod statement;
pub mod types;
mod utils;

pub use bundle::*;
pub use graph::*;
pub use module::*;
pub use statement::*;

pub use swc_common;


pub mod plugin;