use crossbeam::channel::Sender;
use linked_hash_map::LinkedHashMap;
use log::debug;
use std::{
  collections::{HashMap, HashSet},
  sync::{Arc, Mutex},
};
use swc_atoms::JsWord;
use swc_ecma_ast::{
  ArrowExpr, BindingIdent, BlockStmt, BlockStmtOrExpr, CallExpr, CatchClause, ClassDecl, ClassExpr,
  ClassMethod, ClassProp, Constructor, Decl, DefaultDecl, ExportDefaultDecl, Expr, ExprStmt,
  FnDecl, FnExpr, ForInStmt, ForOfStmt, ForStmt, Function, Ident, ImportDecl, ImportNamedSpecifier,
  MemberExpr, MethodProp, ModuleDecl, ModuleItem, ObjectLit, Param, Pat, PatOrExpr, PrivateMethod,
  SetterProp, Stmt, TaggedTpl, Tpl, VarDecl, VarDeclarator,
};
use swc_ecma_visit::{noop_visit_mut_type, VisitAllWith, VisitMut, VisitMutWith};

use crate::{ext::MarkExt, graph::Msg, symbol_box::SymbolBox};

use self::{
  rel::RelationInfo,
  scope::{BindType, Scope, ScopeKind},
};

mod helper;
pub mod rel;
pub mod scope;
mod symbol;
use rel::{DynImportDesc, ExportDesc, ReExportDesc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideEffect {
  Call,
  PropVisit,
  NonRootScope,
  GlobalName,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ModuleItemInfo {
  pub declared: HashSet<JsWord>,
  pub reads: HashSet<JsWord>,
  pub writes: HashSet<JsWord>,
  pub side_effects: Option<SideEffect>,
}

// Declare symbols
// Bind symbols. We use Hoister to handle varible hoisting situation.
// TODO: Fold constants
pub struct Scanner {
  pub cur_relation_order: usize,
  pub statement_infos: Vec<ModuleItemInfo>,
  pub cur_stmt_index: usize,
  // scope
  pub stacks: Vec<Scope>,
  // mark
  pub ident_type: IdentType,
  // relationships between modules.
  pub import_infos: LinkedHashMap<JsWord, RelationInfo>,
  pub local_exports: HashMap<JsWord, ExportDesc>,
  pub re_exports: HashMap<JsWord, ReExportDesc>,
  pub re_export_infos: LinkedHashMap<JsWord, RelationInfo>,
  pub export_all_sources: HashSet<(JsWord, usize)>,
  pub dynamic_imports: HashSet<DynImportDesc>,
  pub symbol_box: Arc<Mutex<SymbolBox>>,
  pub tx: Sender<Msg>,
}

impl Scanner {
  pub fn new(symbol_box: Arc<Mutex<SymbolBox>>, tx: Sender<Msg>) -> Self {
    Self {
      cur_relation_order: 0,
      statement_infos: Default::default(),
      cur_stmt_index: 0,
      // scope
      stacks: vec![Scope::new(ScopeKind::Fn)],
      local_exports: Default::default(),
      re_exports: Default::default(),
      re_export_infos: Default::default(),
      export_all_sources: Default::default(),
      dynamic_imports: Default::default(),
      import_infos: Default::default(),
      ident_type: IdentType::Ref,
      symbol_box,
      tx,
    }
  }

  pub fn declare(&mut self, id: &mut Ident, kind: BindType) {
    let is_var_decl = matches!(kind, BindType::Var);
    let finded_scope = self.stacks.iter_mut().enumerate().rev().find(|(_, scope)| {
      if is_var_decl {
        scope.kind == ScopeKind::Fn
      } else {
        true
      }
    });

    if let Some((idx, scope)) = finded_scope {
      let is_root_scope = idx == 0;
      let declared_name = &id.sym;
      if let Some(declared_kind) = scope.declared_symbols_kind.get(declared_name) {
        // Valid
        // var a; var a;
        assert!(
          declared_kind == &BindType::Var && is_var_decl,
          " duplicate name {}",
          declared_name
        );
      }

      let mark = self.symbol_box.lock().unwrap().new_mark();

      log::debug!(
        "[scanner]: new mark {:?} for `{}` is_root_scope: {:#}",
        mark,
        id.sym.to_string(),
        idx == 0
      );

      scope
        .declared_symbols_kind
        .insert(declared_name.clone(), kind);
      scope.declared_symbols.insert(declared_name.clone(), mark);
      scope.declared_symbols.insert(id.sym.clone(), mark);
      id.span.ctxt = mark.as_ctxt();

      let module_item_info = &mut self.statement_infos[self.cur_stmt_index];
      if is_root_scope {
        // TODO: duplicate detect
        module_item_info.declared.insert(id.sym.clone());
      };
    }
  }

  pub fn resolve_ctxt_for_ident(&mut self, ident: &mut Ident) {
    let mut is_finded = false;
    for (idx, scope) in &mut self.stacks.iter_mut().enumerate().rev() {
      let is_root_scope = idx == 0;
      if let Some(mark) = scope.declared_symbols.get(&ident.sym) {
        ident.span.ctxt = mark.as_ctxt();
        is_finded = true;
        if is_root_scope {
          let stmt_info = &mut self.statement_infos[self.cur_stmt_index];
          // TODO: duplicate detect
          stmt_info.reads.insert(ident.sym.clone());
        }
        break;
      };
    }
    if !is_finded {
      let stmt_info = &mut self.statement_infos[self.cur_stmt_index];
      stmt_info.reads.insert(ident.sym.clone());
    }
  }

  fn visit_mut_stmt_within_child_scope(&mut self, s: &mut Stmt) {
    let scope = Scope::new(ScopeKind::Block);
    self.stacks.push(scope);
    self.visit_mut_stmt_within_same_scope(s);
    self.stacks.pop();
  }

  fn visit_mut_stmt_within_same_scope(&mut self, s: &mut Stmt) {
    match s {
      Stmt::Block(s) => {
        s.visit_mut_children_with(self);
      }
      _ => s.visit_mut_with(self),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentType {
  Binding(BindType),
  Ref,
  Label,
}

impl VisitMut for Scanner {
  noop_visit_mut_type!();

  fn visit_mut_module(&mut self, node: &mut swc_ecma_ast::Module) {
    self.statement_infos = vec![Default::default(); node.body.len()];
    let mut hoister = Hoister::new(self);
    node.visit_mut_children_with(&mut hoister);
    node.visit_mut_children_with(self);
  }

  fn visit_mut_module_item(&mut self, node: &mut swc_ecma_ast::ModuleItem) {
    if let ModuleItem::Stmt(stmt) = node {
      match stmt {
        Stmt::Expr(expr_stmt) => match expr_stmt.expr.as_ref() {
          Expr::Call(_) => {
            self.statement_infos[self.cur_stmt_index].side_effects = Some(SideEffect::Call);
          }
          Expr::Object(_) => {
            self.statement_infos[self.cur_stmt_index].side_effects = Some(SideEffect::PropVisit);
          }
          _ => {}
        },
        Stmt::Block(_) => {
          self.statement_infos[self.cur_stmt_index].side_effects = Some(SideEffect::NonRootScope);
        }
        _ => {}
      }
    }

    node.visit_mut_children_with(self);
    self.cur_stmt_index += 1;
  }

  fn visit_mut_module_decl(&mut self, node: &mut ModuleDecl) {
    self.add_import(node);
    self.add_export(node);

    node.visit_mut_children_with(self);
  }

  fn visit_mut_call_expr(&mut self, node: &mut CallExpr) {
    self.add_dynamic_import(node);

    node.visit_mut_children_with(self);
  }

  #[inline]
  fn visit_mut_import_decl(&mut self, _: &mut ImportDecl) {
    // We alreay done this in Hoister
    // self.ident_type = IdentType::Binding(BindType::Const);
    // n.visit_mut_children_with(self);
  }

  fn visit_mut_arrow_expr(&mut self, e: &mut ArrowExpr) {
    // let child_mark = Mark::fresh(Mark::root());

    self.push_scope(ScopeKind::Fn);

    let old = self.ident_type;
    self.ident_type = IdentType::Binding(BindType::Var);
    e.params.visit_mut_with(self);
    self.ident_type = old;
    match &mut e.body {
      BlockStmtOrExpr::BlockStmt(s) => s.stmts.visit_mut_with(self),
      BlockStmtOrExpr::Expr(e) => e.visit_mut_with(self),
    }
    self.pop_scope();
  }

  fn visit_mut_binding_ident(&mut self, i: &mut BindingIdent) {
    let ident_type = self.ident_type;

    self.ident_type = ident_type;
    i.id.visit_mut_with(self);
    // FIXME: what???
    self.ident_type = ident_type;
  }

  fn visit_mut_block_stmt(&mut self, block: &mut BlockStmt) {
    self.push_scope(ScopeKind::Block);
    block.visit_mut_children_with(self);
    self.pop_scope();
  }

  /// Handle body of the arrow functions
  fn visit_mut_block_stmt_or_expr(&mut self, node: &mut BlockStmtOrExpr) {
    match node {
      BlockStmtOrExpr::BlockStmt(block) => block.visit_mut_children_with(self).into(),
      BlockStmtOrExpr::Expr(e) => e.visit_mut_with(self).into(),
    }
  }

  fn visit_mut_catch_clause(&mut self, c: &mut CatchClause) {
    self.push_scope(ScopeKind::Block);

    self.ident_type = IdentType::Binding(BindType::Var);
    c.param.visit_mut_with(self);
    self.ident_type = IdentType::Ref;

    c.body.visit_mut_children_with(self);
    self.pop_scope();
  }

  fn visit_mut_class_decl(&mut self, n: &mut ClassDecl) {
    self.declare(&mut n.ident, BindType::Let);

    self.push_scope(ScopeKind::Fn);

    self.ident_type = IdentType::Ref;

    n.class.visit_mut_with(self);

    self.pop_scope();
  }

  fn visit_mut_class_expr(&mut self, n: &mut ClassExpr) {
    self.push_scope(ScopeKind::Fn);

    self.ident_type = IdentType::Binding(BindType::Var);
    n.ident.visit_mut_with(self);
    self.ident_type = IdentType::Ref;

    n.class.visit_mut_with(self);

    self.pop_scope();
  }

  fn visit_mut_class_method(&mut self, m: &mut ClassMethod) {
    m.key.visit_mut_with(self);

    self.push_scope(ScopeKind::Fn);

    m.function.visit_mut_with(self);

    self.pop_scope();
  }

  fn visit_mut_class_prop(&mut self, p: &mut ClassProp) {
    p.decorators.visit_mut_with(self);

    if p.key.is_computed() {
      let old = self.ident_type;
      self.ident_type = IdentType::Binding(BindType::Var);
      p.key.visit_mut_with(self);
      self.ident_type = old;
    }

    let old = self.ident_type;
    self.ident_type = IdentType::Ref;
    p.value.visit_mut_with(self);
    self.ident_type = old;

    // p.type_ann.visit_mut_with(self);
  }

  fn visit_mut_constructor(&mut self, c: &mut Constructor) {
    self.push_scope(ScopeKind::Fn);

    let old = self.ident_type;
    self.ident_type = IdentType::Binding(BindType::Var);
    c.params.visit_mut_with(self);
    self.ident_type = old;

    match &mut c.body {
      Some(body) => {
        body.visit_mut_children_with(self);
      }
      None => {}
    }

    self.pop_scope();
  }

  fn visit_mut_decl(&mut self, decl: &mut Decl) {
    decl.visit_mut_children_with(self)
  }

  fn visit_mut_export_default_decl(&mut self, e: &mut ExportDefaultDecl) {
    // Treat default exported functions and classes as declarations
    // even though they are parsed as expressions.
    match &mut e.decl {
      DefaultDecl::Fn(f) => {
        if f.ident.is_some() {
          self.push_scope(ScopeKind::Fn);
          f.function.visit_mut_with(self);
          self.pop_scope();
        } else {
          f.visit_mut_with(self)
        }
      }
      DefaultDecl::Class(c) => {
        // Skip class expression visitor to treat as a declaration.
        c.class.visit_mut_with(self)
      }
      _ => e.visit_mut_children_with(self),
    }
  }

  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    let old = self.ident_type;
    self.ident_type = IdentType::Ref;
    expr.visit_mut_children_with(self);
    self.ident_type = old;
  }

  fn visit_mut_fn_decl(&mut self, node: &mut FnDecl) {
    self.push_scope(ScopeKind::Fn);

    node.function.visit_mut_with(self);

    self.pop_scope();
  }

  fn visit_mut_fn_expr(&mut self, e: &mut FnExpr) {
    self.push_scope(ScopeKind::Fn);

    if let Some(ident) = &mut e.ident {
      self.declare(ident, BindType::Var);
    }
    e.function.visit_mut_with(self);

    self.pop_scope();
  }

  fn visit_mut_for_in_stmt(&mut self, n: &mut ForInStmt) {
    self.push_scope(ScopeKind::Block);

    n.left.visit_mut_with(self);
    n.right.visit_mut_with(self);

    self.visit_mut_stmt_within_child_scope(&mut *n.body);

    self.pop_scope();
  }

  fn visit_mut_for_of_stmt(&mut self, n: &mut ForOfStmt) {
    self.push_scope(ScopeKind::Block);

    n.left.visit_mut_with(self);
    n.right.visit_mut_with(self);

    self.visit_mut_stmt_within_child_scope(&mut *n.body);
    self.pop_scope();
  }

  fn visit_mut_for_stmt(&mut self, n: &mut ForStmt) {
    self.push_scope(ScopeKind::Block);

    n.init.visit_mut_with(self);
    self.ident_type = IdentType::Ref;
    n.test.visit_mut_with(self);
    self.ident_type = IdentType::Ref;
    n.update.visit_mut_with(self);

    self.visit_mut_stmt_within_same_scope(&mut *n.body);

    self.pop_scope();
  }

  fn visit_mut_function(&mut self, f: &mut Function) {
    self.ident_type = IdentType::Ref;
    f.decorators.visit_mut_with(self);

    self.ident_type = IdentType::Binding(BindType::Var);
    f.params.visit_mut_with(self);

    self.ident_type = IdentType::Ref;
    match &mut f.body {
      Some(body) => {
        // Prevent creating new scope.
        body.visit_mut_children_with(self);
      }
      None => {}
    }
  }

  fn visit_mut_ident(&mut self, i: &mut Ident) {
    match self.ident_type {
      IdentType::Binding(kind) => self.declare(i, kind),
      IdentType::Ref => {
        self.resolve_ctxt_for_ident(i);
      }
      // We currently does not touch labels
      IdentType::Label => {}
    }
  }

  fn visit_mut_import_named_specifier(&mut self, s: &mut ImportNamedSpecifier) {
    let old = self.ident_type;
    self.ident_type = IdentType::Binding(BindType::Const);
    s.local.visit_mut_with(self);
    self.ident_type = old;
  }

  /// Leftmost one of a member expression should be resolved.
  fn visit_mut_member_expr(&mut self, e: &mut MemberExpr) {
    e.obj.visit_mut_with(self);

    if e.prop.is_computed() {
      e.prop.visit_mut_with(self);
    }
  }

  // TODO: How should I handle this?
  // typed!(visit_mut_ts_namespace_export_decl, TsNamespaceExportDecl);

  // track_ident_mut!();

  fn visit_mut_method_prop(&mut self, m: &mut MethodProp) {
    m.key.visit_mut_with(self);

    {
      self.push_scope(ScopeKind::Fn);

      m.function.visit_mut_with(self);
      self.pop_scope();
    };
  }

  fn visit_mut_module_items(&mut self, stmts: &mut Vec<ModuleItem>) {
    stmts.visit_mut_children_with(self);
  }

  fn visit_mut_object_lit(&mut self, o: &mut ObjectLit) {
    self.push_scope(ScopeKind::Block);

    o.visit_mut_children_with(self);

    self.pop_scope();
  }

  fn visit_mut_param(&mut self, param: &mut Param) {
    self.ident_type = IdentType::Binding(BindType::Var);
    param.visit_mut_children_with(self);
  }

  fn visit_mut_pat(&mut self, p: &mut Pat) {
    p.visit_mut_children_with(self);
  }

  fn visit_mut_private_method(&mut self, m: &mut PrivateMethod) {
    m.key.visit_mut_with(self);

    self.push_scope(ScopeKind::Fn);
    m.function.visit_mut_with(self);
    self.pop_scope();
  }

  // fn visit_mut_private_name(&mut self, _: &mut PrivateName) {}

  fn visit_mut_setter_prop(&mut self, n: &mut SetterProp) {
    n.key.visit_mut_with(self);

    self.push_scope(ScopeKind::Fn);
    self.ident_type = IdentType::Binding(BindType::Var);
    n.param.visit_mut_with(self);
    n.body.visit_mut_with(self);
    self.pop_scope();
  }

  fn visit_mut_stmts(&mut self, stmts: &mut Vec<Stmt>) {
    stmts.visit_mut_children_with(self)
  }

  fn visit_mut_var_decl(&mut self, decl: &mut VarDecl) {
    let ident_type = self.ident_type;
    self.ident_type = IdentType::Binding(decl.kind.into());
    decl.decls.visit_mut_with(self);
    self.ident_type = ident_type;
  }

  fn visit_mut_var_declarator(&mut self, decl: &mut VarDeclarator) {
    decl.name.visit_mut_with(self);

    let old_type = self.ident_type;
    self.ident_type = IdentType::Ref;
    decl.init.visit_mut_children_with(self);
    self.ident_type = old_type;
  }
}

// TODO: handle `var foo = 1`
pub struct Hoister<'me> {
  scanner: &'me mut Scanner,
  ident_type: Option<IdentType>,
  /// Hoister should not touch let / const in the block.
  in_block: bool,
  catch_param_decls: HashSet<JsWord>,
}

impl<'me> Hoister<'me> {
  pub fn new(scanner: &'me mut Scanner) -> Self {
    Self {
      scanner,
      ident_type: None,
      in_block: false,
      catch_param_decls: Default::default(),
    }
  }
}

impl<'me> VisitMut for Hoister<'me> {
  noop_visit_mut_type!();

  // const foo = () => {}
  #[inline]
  fn visit_mut_arrow_expr(&mut self, _: &mut ArrowExpr) {}

  // We only care about declarations.
  #[inline]
  fn visit_mut_expr(&mut self, _: &mut Expr) {}

  // new Foo()
  #[inline]
  fn visit_mut_constructor(&mut self, _: &mut Constructor) {}

  // function foo() {}
  #[inline]
  fn visit_mut_function(&mut self, _: &mut Function) {}

  #[inline]
  fn visit_mut_param(&mut self, _: &mut Param) {}

  #[inline]
  fn visit_mut_pat_or_expr(&mut self, _: &mut PatOrExpr) {}

  // { get foo() {} }
  #[inline]
  fn visit_mut_setter_prop(&mut self, _: &mut SetterProp) {}

  // css``
  #[inline]
  fn visit_mut_tagged_tpl(&mut self, _: &mut TaggedTpl) {}

  #[inline]
  fn visit_mut_tpl(&mut self, _: &mut Tpl) {}

  fn visit_mut_module_item(&mut self, node: &mut swc_ecma_ast::ModuleItem) {
    node.visit_mut_children_with(self);
    self.scanner.cur_stmt_index += 1;
  }

  fn visit_mut_module_items(&mut self, stmts: &mut Vec<ModuleItem>) {
    stmts.visit_mut_children_with(self);
    self.scanner.cur_stmt_index = 0;
  }

  fn visit_mut_import_decl(&mut self, n: &mut ImportDecl) {
    let prev = self.ident_type;
    self.ident_type = Some(IdentType::Binding(BindType::Import));
    n.visit_mut_children_with(self);
    self.ident_type = prev;
  }

  fn visit_mut_import_named_specifier(&mut self, s: &mut ImportNamedSpecifier) {
    // let old = self.ident_type;
    // self.ident_type = IdentType::Binding(BindType::Const);
    // For `import { foo as foo2 }`, We only need to mark `foo2`.
    s.local.visit_mut_with(self);
    // self.ident_type = old;
  }

  fn visit_mut_ident(&mut self, i: &mut Ident) {
    if let Some(ident_type) = &self.ident_type {
      match ident_type {
        IdentType::Binding(kind) => self.scanner.declare(i, kind.clone()),
        // We only bind symbol in Hoister
        _ => {}
      }
    }
  }

  // function foo() {};
  fn visit_mut_fn_decl(&mut self, node: &mut FnDecl) {
    if self.catch_param_decls.contains(&node.ident.sym) {
      return;
    }
    self.scanner.declare(&mut node.ident, BindType::Var);
  }

  // fn visit_mut_assign_pat_prop(&mut self, node: &mut AssignPatProp) {
  //     // node.visit_mut_children_with(self);

  //     // {
  //     //     if self.catch_param_decls.contains(&node.key.sym) {
  //     //         return;
  //     //     }

  //     //     self.resolver.modify(&mut node.key, self.kind)
  //     // }
  // }

  // fn visit_mut_block_stmt(&mut self, n: &mut BlockStmt) {
  //     let old_in_block = self.in_block;
  //     self.in_block = true;
  //     n.visit_mut_children_with(self);
  //     self.in_block = old_in_block;
  // }

  // #[inline]
  // fn visit_mut_catch_clause(&mut self, c: &mut CatchClause) {
  //     // let params: Vec<Id> = find_ids(&c.param);

  //     // let orig = self.catch_param_decls.clone();

  //     // self.catch_param_decls
  //     //     .extend(params.into_iter().map(|v| v.0));
  //     c.body.visit_mut_with(self);

  //     // self.catch_param_decls = orig;
  // }

  // fn visit_mut_class_decl(&mut self, node: &mut ClassDecl) {
  //     if self.in_block {
  //         return;
  //     }
  //     // self.resolver.in_type = false;
  //     // self.resolver
  //     //     .modify(&mut node.ident, Some(BindType::Let));
  // }

  // fn visit_mut_export_default_decl(&mut self, node: &mut ExportDefaultDecl) {
  //     // Treat default exported functions and classes as declarations
  //     // even though they are parsed as expressions.
  //     match &mut node.decl {
  //         DefaultDecl::Fn(f) => {
  //             if let Some(id) = &mut f.ident {
  //                 self.resolver.in_type = false;
  //                 self.resolver.modify(id, Some(BindType::Var));
  //             }

  //             f.visit_mut_with(self)
  //         }
  //         DefaultDecl::Class(c) => {
  //             // if let Some(id) = &mut c.ident {
  //             //     self.resolver.in_type = false;
  //             //     self.resolver.modify(id, Some(BindType::Let));
  //             // }

  //             c.visit_mut_with(self)
  //         }
  //         _ => {
  //             node.visit_mut_children_with(self);
  //         }
  //     }
  // }

  // fn visit_mut_fn_decl(&mut self, node: &mut FnDecl) {
  //     if self.catch_param_decls.contains(&node.ident.sym) {
  //         return;
  //     }

  //     self.resolver.in_type = false;
  //     self.resolver
  //         .modify(&mut node.ident, Some(BindType::Var));
  // }

  // fn visit_mut_pat(&mut self, node: &mut Pat) {
  //     self.resolver.in_type = false;
  //     match node {
  //         Pat::Ident(i) => {
  //             if self.catch_param_decls.contains(&i.id.sym) {
  //                 return;
  //             }

  //             self.resolver.modify(&mut i.id, self.kind)
  //         }

  //         _ => node.visit_mut_children_with(self),
  //     }
  // }

  // fn visit_mut_var_decl(&mut self, node: &mut VarDecl) {
  //     if self.in_block {
  //         match node.kind {
  //             BindType::Const | BindType::Let => return,
  //             _ => {}
  //         }
  //     }

  //     let old_kind = self.kind;
  //     self.kind = Some(node.kind);

  //     self.resolver.hoist = false;

  //     node.visit_mut_children_with(self);

  //     self.kind = old_kind;
  // }

  // fn visit_mut_var_decl_or_expr(&mut self, n: &mut VarDeclOrExpr) {
  //     match n {
  //         VarDeclOrExpr::VarDecl(VarDecl {
  //             kind: BindType::Let,
  //             ..
  //         })
  //         | VarDeclOrExpr::VarDecl(VarDecl {
  //             kind: BindType::Const,
  //             ..
  //         }) => {}
  //         _ => {
  //             n.visit_mut_children_with(self);
  //         }
  //     }
  // }

  // fn visit_mut_var_decl_or_pat(&mut self, n: &mut VarDeclOrPat) {
  //     match n {
  //         VarDeclOrPat::VarDecl(VarDecl {
  //             kind: BindType::Let,
  //             ..
  //         })
  //         | VarDeclOrPat::VarDecl(VarDecl {
  //             kind: BindType::Const,
  //             ..
  //         }) => {}
  //         // Hoister should not handle lhs of for in statement below
  //         //
  //         // const b = [];
  //         // {
  //         //   let a;
  //         //   for (a in b) {
  //         //     console.log(a);
  //         //   }
  //         // }
  //         VarDeclOrPat::Pat(..) => {}
  //         _ => {
  //             n.visit_mut_children_with(self);
  //         }
  //     }
  // }

  // #[inline]
  // fn visit_mut_var_declarator(&mut self, node: &mut VarDeclarator) {
  //     node.name.visit_mut_with(self);
  // }
}
