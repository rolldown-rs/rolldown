// #[macro_use]
// extern crate swc_common;
// extern crate swc_ecma_parser;

use std::cell::RefCell;
use std::rc::Rc;

use swc_common::sync::Lrc;
use swc_common::{
  errors::{ColorConfig, Handler},
  FileName, FilePathMapping, SourceMap,
};
use swc_ecma_ast::Module;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};

pub fn parse_to_ast(codes: String) -> Rc<RefCell<Module>> {
  let cm: Lrc<SourceMap> = Default::default();
  // let handler =
  //       Handler::with_emitter(ColorConfig::Auto, true, false,
  //       Some(cm.clone()));
  // Real usage
  // let fm = cm
  //     .load_file(Path::new("test.js"))
  //     .expect("failed to load test.js");
  let fm = cm.new_source_file(FileName::Custom("TODO: real filename ?".into()), codes);
  let lexer = Lexer::new(
    // We want to parse ecmascript
    Syntax::Es(Default::default()),
    // JscTarget defaults to es5
    Default::default(),
    StringInput::from(&*fm),
    None,
  );

  let mut parser = Parser::new_from(lexer);

  // for e in parser.take_errors() {
  //     e.into_diagnostic(&handler).emit();
  // }

  let module = parser
    .parse_module()
    // .map_err(|mut e| {
    //     // Unrecoverable fatal error occurred
    //     e.into_diagnostic(&handler).emit()
    // })
    .expect("failed to parser module");
  // module.body
  Rc::new(RefCell::new(module))
}

#[cfg(test)]
mod tests {
  use super::*;
  // #[test]
  // fn e2e() {

  //   let codes = std::fs::read_to_string("./demo/main.js").unwrap();

  //   let ast = parse_to_ast(codes);
  //   // module.body
  //   // println!("module: {:?}", ast.borrow().body);
  //   // println!("imports: {:?}", module.borrow().imports);
  // }
}
