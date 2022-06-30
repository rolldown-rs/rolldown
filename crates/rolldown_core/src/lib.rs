#![feature(iter_intersperse)]

mod options;
use ast::Id;
use hashbrown::HashMap;
pub use options::*;
mod graph;
pub use graph::*;
mod module;
pub use module::*;
mod resolving_module_job;
pub use resolving_module_job::*;
mod plugin;
pub use plugin::*;
mod utils;
use swc_atoms::JsWord;
pub use utils::*;
mod visitors;
pub use visitors::*;
mod bundle;
pub use bundle::*;
mod chunk;
pub use chunk::*;
mod chunk_spliter;
pub use chunk_spliter::*;
mod chunk_graph;
pub use chunk_graph::*;
mod ufriend;
pub(crate) use ufriend::*;

pub type ModuleById = HashMap<JsWord, Module>;
pub type LocalExports = HashMap<JsWord, Id>;
pub type MergedExports = HashMap<JsWord, Id>;

#[derive(Debug, Clone)]
pub struct ResolvedId {
    pub id: JsWord,
    pub external: bool,
}

impl ResolvedId {
    pub fn new<T: Into<JsWord>>(id: T, external: bool) -> Self {
        Self { id: id.into(), external }
    }
}
