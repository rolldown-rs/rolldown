// #[macro_use]
// extern crate swc_common;
// extern crate swc_ecma_parser;

use std::cell::RefCell;
use std::rc::Rc;
use std::{collections::HashMap, io::stdout};
use swc_common::{
  errors::{ColorConfig, Handler},
  sync::Lrc,
  FileName, FilePathMapping, SourceMap,
};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};

// pub fn parse_to_ast(module: &crate::module::Module) -> swc_ecma_ast::Module {
//   // let handler =
//   //       Handler::with_emitter(ColorConfig::Auto, true, false,
//   //       Some(cm.clone()));
//   // Real usage
//   // let fm = cm
//   //     .load_file(Path::new("test.js"))
//   //     .expect("failed to load test.js");

//   let fm = cm.new_source_file(FileName::Custom(module.id.clone()), module.source);
//   let lexer = Lexer::new(
//     // We want to parse ecmascript
//     Syntax::Es(Default::default()),
//     // JscTarget defaults to es5
//     Default::default(),
//     StringInput::from(&*fm),
//     None,
//   );

//   let mut parser = Parser::new_from(lexer);

//   // for e in parser.take_errors() {
//   //     e.into_diagnostic(&handler).emit();
//   // }

//   let module = parser
//     .parse_module()
//     // .map_err(|mut e| {
//     //     // Unrecoverable fatal error occurred
//     //     e.into_diagnostic(&handler).emit()
//     // })
//     .expect("failed to parser module");
//   // module.body
//   module
// }

// pub fn ast_to_source(node: &Module) {
//   let cm = Lrc::new(SourceMap::new(FilePathMapping::empty()));

//   let wr = stdout();
//   let mut emitter = Emitter {
//     cfg: swc_ecma_codegen::Config { minify: false },
//     cm: cm.clone(),
//     comments: None,
//     wr: Box::new(JsWriter::new(cm.clone(), "\n", wr.lock(), None)),
//   };
//   emitter.emit_module(node).unwrap();
// }

// #[cfg(test)]
// mod tests {
//   use super::*;
//   #[test]
//   fn e2e() {
//     let codes = std::fs::read_to_string("./demo/main.js").unwrap();

//     let ast = parse_to_ast(codes);
//     // module.body
//     // println!("module: {:?}", ast.to);
//     // println!("imports: {:?}", module.borrow().imports);
//   }
// }
