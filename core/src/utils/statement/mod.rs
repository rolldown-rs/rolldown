use swc_common::DUMMY_SP;
use swc_ecma_ast::{EmptyStmt, ModuleItem, Stmt};

pub fn create_empty_statement() -> ModuleItem {
  ModuleItem::Stmt(Stmt::Empty(EmptyStmt { span: DUMMY_SP }))
}
