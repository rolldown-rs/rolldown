pub mod bundle;
pub mod chunk;
pub mod external_module;
pub mod graph;
// pub mod linker;
pub mod module;
pub mod scanner;
// pub mod statement;
pub mod renamer;
pub mod types;
pub mod utils;
pub mod worker;

pub use swc_ecma_ast as ast;

// refactor
pub mod ext;
pub mod plugin_driver;
pub mod symbol_box;
pub mod compiler;
pub mod statement;