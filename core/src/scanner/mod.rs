use log::debug;
use std::collections::{HashMap, HashSet};
use swc_atoms::JsWord;
use swc_common::{Mark, SyntaxContext};
use swc_ecma_ast::{
  ArrowExpr, BindingIdent, BlockStmt, BlockStmtOrExpr, CallExpr, CatchClause, ClassDecl, ClassExpr,
  ClassMethod, ClassProp, Constructor, Decl, DefaultDecl, ExportDefaultDecl, Expr, ExprOrSuper,
  FnDecl, FnExpr, ForInStmt, ForOfStmt, ForStmt, Function, Ident, ImportDecl, ImportNamedSpecifier,
  MemberExpr, MethodProp, ModuleDecl, ModuleItem, ObjectLit, Param, Pat, PrivateMethod, SetterProp,
  Stmt, VarDecl, VarDeclKind, VarDeclarator,
};
use swc_ecma_visit::{noop_visit_mut_type, VisitMut, VisitMutWith};

use self::{
  helper::collect_mut_ident_of_pat,
  scope::{Scope, ScopeKind},
};

mod helper;
pub mod rel;
pub mod scope;
mod symbol;
use rel::{DynImportDesc, ExportDesc, ImportDesc, ReExportDesc};

// Declare symbols
// Bind symbols. We use Hoister to handle varible hoisting situation.
// TODO: Fold constants
pub struct Scanner {
  // scope
  pub stacks: Vec<Scope>,
  pub ident_type: IdentType,
  // relationships between modules.
  pub imports: HashMap<JsWord, ImportDesc>,
  pub exports: HashMap<JsWord, ExportDesc>,
  pub re_exports: HashMap<JsWord, ReExportDesc>,
  pub export_all_sources: HashSet<JsWord>,
  pub dynamic_imports: HashSet<DynImportDesc>,
  pub sources: HashSet<JsWord>,
}

impl Scanner {
  pub fn new(scope: Scope) -> Self {
    Self {
      // scope
      stacks: vec![scope],
      // rel
      imports: Default::default(),
      exports: Default::default(),
      re_exports: Default::default(),
      export_all_sources: Default::default(),
      dynamic_imports: Default::default(),
      sources: Default::default(),
      ident_type: IdentType::Ref,
    }
  }

  pub fn declare(&mut self, id: &mut Ident, kind: VarDeclKind) {
    let is_var_decl = match kind {
      VarDeclKind::Let => false,
      VarDeclKind::Const => false,
      VarDeclKind::Var => true,
    };

    debug!(
      "declare {} {}",
      match kind {
        VarDeclKind::Let => "let",
        VarDeclKind::Const => "const",
        VarDeclKind::Var => "var",
      },
      &id.sym.to_string()
    );

    let cur_mark = self.get_cur_scope().mark;

    for scope in &mut self.stacks.iter_mut().rev() {
      if is_var_decl {
        if scope.kind == ScopeKind::Fn {
          let ctxt = SyntaxContext::empty().apply_mark(Mark::fresh(cur_mark));
          id.span.ctxt = ctxt;
          scope.declared_symbols.insert(id.sym.clone(), ctxt);
        }
      } else {
        let ctxt = SyntaxContext::empty().apply_mark(Mark::fresh(cur_mark));
        id.span.ctxt = ctxt;
        scope.declared_symbols.insert(id.sym.clone(), ctxt);
        break;
      }
    }

    print!("stack {:#?}\n", self.stacks);
  }

  pub fn declare_pat(&mut self, pat: &mut Pat, kind: VarDeclKind) {
    collect_mut_ident_of_pat(pat)
      .into_iter()
      .for_each(|ident| self.declare(ident, kind));
  }

  pub fn resolve_ctxt_for_ident(&mut self, ident: &mut Ident) {
    for scope in &mut self.stacks.iter_mut().rev() {
      if let Some(ctxt) = scope.declared_symbols.get(&ident.sym) {
        ident.span.ctxt = ctxt.clone();
        break;
      };
    }
  }

  pub fn resolve_ctxt_for_pat(&mut self, pat: &mut Pat) {
    collect_mut_ident_of_pat(pat).into_iter().for_each(|id| {
      self.resolve_ctxt_for_ident(id);
    });
  }

  fn visit_mut_stmt_within_child_scope(&mut self, s: &mut Stmt) {
    let scope = Scope::new(ScopeKind::Block, Mark::fresh(Mark::root()));
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
  Binding(VarDeclKind),
  Ref,
  Label,
}

impl VisitMut for Scanner {
  noop_visit_mut_type!();

  // fn visit_mut_ident(&mut self, node: &mut Ident) {
  //   self.resolve_ctxt_for_ident(node);
  // }

  // fn visit_mut_member_expr(&mut self, node: &mut MemberExpr) {
  //   // for `foo.bar.biz` or  'foo.[bar]' we only care about `foo`
  //   if let ExprOrSuper::Expr(exp) = &mut node.obj {
  //     if let Expr::Ident(id) = exp.as_mut() {
  //       self.resolve_ctxt_for_ident(id);
  //     }
  //   }
  // }

  fn visit_mut_module_decl(&mut self, node: &mut ModuleDecl) {
    self.add_import(node);
    self.add_export(node);

    node.visit_mut_children_with(self);
  }

  fn visit_mut_call_expr(&mut self, node: &mut CallExpr) {
    self.add_dynamic_import(node);

    node.visit_mut_children_with(self);
  }

  // fn visit_mut_export_default_decl(&mut self, node: &mut ExportDefaultDecl) {
  //   let id = match &mut node.decl {
  //     DefaultDecl::Class(node) => node.ident.as_mut().map(|s| (s, VarDeclKind::Let)),
  //     DefaultDecl::Fn(node) => node.ident.as_mut().map(|s| (s, VarDeclKind::Var)),
  //     _ => None,
  //   };
  //   if let Some((id, kind)) = id {
  //     self.declare(&id.sym, kind);
  //   }
  //   node.visit_mut_children_with(self);
  // }

  // fn visit_mut_fn_expr(&mut self, node: &mut FnExpr) {
  //   self.push_scope(ScopeKind::Fn);

  //   node.function.params.iter_mut().for_each(|p| {
  //     self.declare_pat(&mut p.pat, VarDeclKind::Var);
  //   });

  //   if let Some(ident) = &mut node.ident {
  //     self.declare(&mut ident.sym, VarDeclKind::Var);
  //   }

  //   self.reuse_scope = true;
  //   self.visit_mut_stmt_within_same_scope(&mut node.function.body)
  //   node.visit_mut_children_with(self);
  //   self.stacks.pop();
  // }

  // fn visit_mut_fn_decl(&mut self, node: &mut FnDecl) {
  // self.declare(&mut node.ident.sym, VarDeclKind::Var);

  // let scope = Scope::new(
  //   ScopeKind::Fn,
  //   Mark::fresh(self.get_cur_scope().mark.clone()),
  // );
  // node.visit_mut_children_with(&mut Hositer { scope: &mut Scope });
  // self.stacks.push(scope);

  // node.function.params.iter_mut().for_each(|p| {
  //   self.declare_pat(&mut p.pat, VarDeclKind::Var);
  // });

  // self.reuse_scope = true;
  // node.visit_mut_children_with(self);
  // self.stacks.pop();
  // }

  // fn visit_mut_arrow_expr(&mut self, node: &mut ArrowExpr) {
  //   let scope = Scope::new(
  //     ScopeKind::Fn,
  //     Mark::fresh(self.get_cur_scope().mark.clone()),
  //   );
  //   self.stacks.push(scope);

  //   node.params.iter_mut().for_each(|p| {
  //     self.declare_pat(p, VarDeclKind::Var);
  //   });

  //   self.reuse_scope = true;
  //   node.visit_mut_children_with(self);
  //   self.stacks.pop();
  // }

  // fn visit_mut_class_method(&mut self, node: &mut ClassMethod) {
  //   let scope = Scope::new(
  //     ScopeKind::Fn,
  //     Mark::fresh(self.get_cur_scope().mark.clone()),
  //   );
  //   self.stacks.push(scope);

  //   node.function.params.iter_mut().for_each(|p| {
  //     self.declare_pat(&mut p.pat, VarDeclKind::Var);
  //   });

  //   self.reuse_scope = true;
  //   node.visit_mut_children_with(self);
  //   self.stacks.pop();
  // }

  // fn visit_mut_method_prop(&mut self, node: &mut MethodProp) {
  //   let scope = Scope::new(
  //     ScopeKind::Fn,
  //     Mark::fresh(self.get_cur_scope().mark.clone()),
  //   );
  //   self.stacks.push(scope);

  //   node.function.params.iter_mut().for_each(|p| {
  //     self.declare_pat(&mut p.pat, VarDeclKind::Var);
  //   });

  //   self.reuse_scope = true;
  //   node.visit_mut_children_with(self);
  //   self.stacks.pop();
  // }

  // fn visit_mut_block_stmt(&mut self, node: &mut BlockStmt) {
  //   self.push_scope(ScopeKind::Block);
  //   node.visit_mut_children_with(self);
  //   self.pop_scope();
  // }

  // fn visit_mut_catch_clause(&mut self, node: &mut CatchClause) {
  //   let scope = Scope::new(
  //     ScopeKind::Block,
  //     Mark::fresh(self.get_cur_scope().mark.clone()),
  //   );
  //   self.stacks.push(scope);
  //   self.reuse_scope = true;
  //   node.visit_mut_children_with(self);
  //   self.stacks.pop();
  // }

  // fn visit_mut_var_decl(&mut self, node: &mut VarDecl) {
  //   node.decls.iter_mut().for_each(|declarator| {
  //     self.declare_pat(&mut declarator.name, node.kind.clone());
  //   });
  // }

  // fn visit_mut_class_decl(&mut self, node: &mut ClassDecl) {
  //   self.declare(&mut node.ident.sym, VarDeclKind::Const);
  //   node.visit_mut_children_with(self);
  // }

  // // --- for

  // fn visit_mut_for_stmt(&mut self, node: &mut ForStmt) {
  //   let scope = Scope::new(
  //     ScopeKind::Block,
  //     Mark::fresh(self.get_cur_scope().mark.clone()),
  //   );
  //   self.stacks.push(scope);
  //   self.reuse_scope = true;
  //   node.visit_mut_children_with(self);
  //   self.stacks.pop();
  // }

  // fn visit_mut_for_in_stmt(&mut self, node: &mut ForInStmt) {
  //   let scope = Scope::new(
  //     ScopeKind::Block,
  //     Mark::fresh(self.get_cur_scope().mark.clone()),
  //   );
  //   self.stacks.push(scope);
  //   self.reuse_scope = true;
  //   node.visit_mut_children_with(self);
  //   self.stacks.pop();
  // }

  // fn visit_mut_for_of_stmt(&mut self, node: &mut ForOfStmt) {
  //   let scope = Scope::new(
  //     ScopeKind::Block,
  //     Mark::fresh(self.get_cur_scope().mark.clone()),
  //   );
  //   self.stacks.push(scope);
  //   self.reuse_scope = true;
  //   node.visit_mut_children_with(self);
  //   self.stacks.pop();
  // }

  fn visit_mut_import_decl(&mut self, n: &mut ImportDecl) {
    self.ident_type = IdentType::Binding(VarDeclKind::Const);
    n.visit_mut_children_with(self);
  }

  fn visit_mut_arrow_expr(&mut self, e: &mut ArrowExpr) {
    // let child_mark = Mark::fresh(Mark::root());

    self.push_scope(ScopeKind::Fn);

    let old = self.ident_type;
    self.ident_type = IdentType::Binding(VarDeclKind::Var);
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
    // let child_mark = Mark::fresh(Mark::root());

    // // Child folder
    // let mut folder = Resolver::new(
    //     Scope::new(ScopeKind::Fn, child_mark, Some(&self.current)),
    //     self.handle_types,
    // );

    self.push_scope(ScopeKind::Block);

    self.ident_type = IdentType::Binding(VarDeclKind::Var);
    c.param.visit_mut_with(self);
    self.ident_type = IdentType::Ref;

    c.body.visit_mut_children_with(self);
    self.pop_scope();
  }

  fn visit_mut_class_decl(&mut self, n: &mut ClassDecl) {
    self.declare(&mut n.ident, VarDeclKind::Let);

    // // Create a child scope. The class name is only accessible within the class.
    // let child_mark = Mark::fresh(Mark::root());

    // let mut folder = Resolver::new(
    //     Scope::new(ScopeKind::Fn, child_mark, Some(&self.current)),
    //     self.handle_types,
    // );
    self.push_scope(ScopeKind::Fn);

    self.ident_type = IdentType::Ref;

    n.class.visit_mut_with(self);

    self.pop_scope();
  }

  fn visit_mut_class_expr(&mut self, n: &mut ClassExpr) {
    // Create a child scope. The class name is only accessible within the class.
    // let child_mark = Mark::fresh(Mark::root());

    // let mut folder = Resolver::new(
    //     Scope::new(ScopeKind::Fn, child_mark, Some(&self.current)),
    //     self.handle_types,
    // );
    self.push_scope(ScopeKind::Fn);

    self.ident_type = IdentType::Binding(VarDeclKind::Var);
    n.ident.visit_mut_with(self);
    self.ident_type = IdentType::Ref;

    n.class.visit_mut_with(self);

    self.pop_scope();
  }

  fn visit_mut_class_method(&mut self, m: &mut ClassMethod) {
    m.key.visit_mut_with(self);

    {
      // let child_mark = Mark::fresh(Mark::root());

      // // Child folder
      // let mut child = Resolver::new(
      //     Scope::new(ScopeKind::Fn, child_mark, Some(&self.current)),
      //     self.handle_types,
      // );

      self.push_scope(ScopeKind::Fn);

      m.function.visit_mut_with(self);

      self.pop_scope();
    }
  }

  fn visit_mut_class_prop(&mut self, p: &mut ClassProp) {
    p.decorators.visit_mut_with(self);

    if p.computed {
      let old = self.ident_type;
      self.ident_type = IdentType::Binding(VarDeclKind::Var);
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
    let child_mark = Mark::fresh(Mark::root());

    // // Child folder
    // let mut folder = Resolver::new(
    //     Scope::new(ScopeKind::Fn, child_mark, Some(&self.current)),
    //     self.handle_types,
    // );

    self.push_scope(ScopeKind::Fn);

    let old = self.ident_type;
    self.ident_type = IdentType::Binding(VarDeclKind::Var);
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
          // let child_mark = Mark::fresh(Mark::root());

          // // Child folder
          // let mut folder = Resolver::new(
          //     Scope::new(ScopeKind::Fn, child_mark, Some(&self.current)),
          //     self.handle_types,
          // );
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
    // self.in_type = false;
    let old = self.ident_type;
    self.ident_type = IdentType::Ref;
    expr.visit_mut_children_with(self);
    self.ident_type = old;
  }

  fn visit_mut_fn_decl(&mut self, node: &mut FnDecl) {
    // We don't fold this as Hoister handles this.

    {
      // let child_mark = Mark::fresh(Mark::root());

      // // Child folder
      // let mut folder = Resolver::new(
      //     Scope::new(ScopeKind::Fn, child_mark, Some(&self.current)),
      //     self.handle_types,
      // );

      self.push_scope(ScopeKind::Fn);

      node.function.visit_mut_with(self);

      self.pop_scope();
    }
  }

  fn visit_mut_fn_expr(&mut self, e: &mut FnExpr) {
    // let child_mark = Mark::fresh(Mark::root());

    // // Child folder
    // let mut folder = Resolver::new(
    //     Scope::new(ScopeKind::Fn, child_mark, Some(&self.current)),
    //     self.handle_types,
    // );

    self.push_scope(ScopeKind::Fn);

    if let Some(ident) = &mut e.ident {
      self.declare(ident, VarDeclKind::Var);
    }
    e.function.visit_mut_with(self);

    self.pop_scope();
  }

  fn visit_mut_for_in_stmt(&mut self, n: &mut ForInStmt) {
    // let child_mark = Mark::fresh(Mark::root());
    // let mut child = Resolver::new(
    //     Scope::new(ScopeKind::Block, child_mark, Some(&self.current)),
    //     self.handle_types,
    // );
    self.push_scope(ScopeKind::Block);

    n.left.visit_mut_with(self);
    n.right.visit_mut_with(self);

    self.visit_mut_stmt_within_child_scope(&mut *n.body);

    self.pop_scope();
  }

  fn visit_mut_for_of_stmt(&mut self, n: &mut ForOfStmt) {
    // let child_mark = Mark::fresh(Mark::root());
    // let mut child = Resolver::new(
    //     Scope::new(ScopeKind::Block, child_mark, Some(&self.current)),
    //     self.handle_types,
    // );

    self.push_scope(ScopeKind::Block);

    n.left.visit_mut_with(self);
    n.right.visit_mut_with(self);

    self.visit_mut_stmt_within_child_scope(&mut *n.body);
    self.pop_scope();
  }

  fn visit_mut_for_stmt(&mut self, n: &mut ForStmt) {
    let child_mark = Mark::fresh(Mark::root());
    self.push_scope(ScopeKind::Block);

    // self.ident_type = IdentType::Binding;
    n.init.visit_mut_with(self);
    self.ident_type = IdentType::Ref;
    n.test.visit_mut_with(self);
    self.ident_type = IdentType::Ref;
    n.update.visit_mut_with(self);

    self.visit_mut_stmt_within_same_scope(&mut *n.body);

    self.pop_scope();
  }

  fn visit_mut_function(&mut self, f: &mut Function) {
    // f.type_params.visit_mut_with(self);

    // self.in_type = false;
    self.ident_type = IdentType::Ref;
    f.decorators.visit_mut_with(self);

    self.ident_type = IdentType::Binding(VarDeclKind::Var);
    f.params.visit_mut_with(self);

    // f.return_type.visit_mut_with(self);

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
        // let Ident { span, sym, .. } = i;

        // // if cfg!(debug_assertions) && LOG {
        // //     eprintln!(
        // //         "resolver: IdentRef (type = {}) {}{:?}",
        // //         self.in_type,
        // //         sym,
        // //         span.ctxt()
        // //     );
        // // }

        // if span.ctxt() != SyntaxContext::empty() {
        //     return;
        // }

        // if let Some(mark) = self.mark_for_ref(&sym) {
        //     let span = span.apply_mark(mark);

        //     if cfg!(debug_assertions) && LOG {
        //         eprintln!("\t -> {:?}", span.ctxt());
        //     }
        //     i.span = span;
        // } else {
        //     if cfg!(debug_assertions) && LOG {
        //         eprintln!("\t -> Unresolved");
        //     }

        //     let mark = {
        //         let mut mark = self.current.mark;
        //         let mut cur = Some(&self.current);
        //         while let Some(scope) = cur {
        //             cur = scope.parent;

        //             if cur.is_none() {
        //                 break;
        //             }

        //             if let Some(cur) = &cur {
        //                 mark = cur.mark;
        //             }
        //         }

        //         mark
        //     };

        //     let span = span.apply_mark(mark);

        //     if cfg!(debug_assertions) && LOG {
        //         eprintln!("\t -> {:?}", span.ctxt());
        //     }

        //     i.span = span;
        //     // Support hoisting
        //     self.modify(i, None)
        // }
      }
      // We currently does not touch labels
      IdentType::Label => {}
    }
  }

  fn visit_mut_import_named_specifier(&mut self, s: &mut ImportNamedSpecifier) {
    let old = self.ident_type;
    self.ident_type = IdentType::Binding(VarDeclKind::Const);
    s.local.visit_mut_with(self);
    self.ident_type = old;
  }

  /// Leftmost one of a member expression should be resolved.
  fn visit_mut_member_expr(&mut self, e: &mut MemberExpr) {
    e.obj.visit_mut_with(self);

    if e.computed {
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
    // if !self.in_ts_module && self.current.kind != ScopeKind::Fn {
    //     return stmts.visit_mut_children_with(self);
    // }
    stmts.visit_mut_children_with(self);

    // // Phase 1: Handle hoisting
    // {
    //     let mut hoister = Hoister {
    //         resolver: self,
    //         kind: None,
    //         in_block: false,
    //         catch_param_decls: Default::default(),
    //     };
    //     stmts.visit_mut_children_with(&mut hoister)
    // }

    // // Phase 2.
    // stmts.visit_mut_children_with(self)
  }

  fn visit_mut_object_lit(&mut self, o: &mut ObjectLit) {
    // let child_mark = Mark::fresh(Mark::root());

    // let mut child_folder = Resolver::new(
    //     Scope::new(ScopeKind::Block, child_mark, Some(&self.current)),
    //     self.handle_types,
    // );
    self.push_scope(ScopeKind::Block);

    o.visit_mut_children_with(self);

    self.pop_scope();
  }

  fn visit_mut_param(&mut self, param: &mut Param) {
    self.ident_type = IdentType::Binding(VarDeclKind::Var);
    param.visit_mut_children_with(self);
  }

  fn visit_mut_pat(&mut self, p: &mut Pat) {
    p.visit_mut_children_with(self);
  }

  fn visit_mut_private_method(&mut self, m: &mut PrivateMethod) {
    m.key.visit_mut_with(self);

    {
      // let child_mark = Mark::fresh(Mark::root());

      // // Child folder
      // let mut child = Resolver::new(
      //     Scope::new(ScopeKind::Fn, child_mark, Some(&self.current)),
      //     self.handle_types,
      // );
      self.push_scope(ScopeKind::Fn);

      m.function.visit_mut_with(self);
      self.pop_scope();
    }
  }

  // fn visit_mut_private_name(&mut self, _: &mut PrivateName) {}

  fn visit_mut_setter_prop(&mut self, n: &mut SetterProp) {
    n.key.visit_mut_with(self);

    {
      // let child_mark = Mark::fresh(Mark::root());

      // // Child folder
      // let mut child = Resolver::new(
      //     Scope::new(ScopeKind::Fn, child_mark, Some(&self.current)),
      //     self.handle_types,
      // );
      self.push_scope(ScopeKind::Fn);

      // self.in_type = false;
      self.ident_type = IdentType::Binding(VarDeclKind::Var);
      n.param.visit_mut_with(self);
      n.body.visit_mut_with(self);
      self.pop_scope();
    };
  }

  fn visit_mut_stmts(&mut self, stmts: &mut Vec<Stmt>) {
    // // Phase 1: Handle hoisting
    // {
    //     let mut hoister = Hoister {
    //         resolver: self,
    //         kind: None,
    //         in_block: false,
    //         catch_param_decls: Default::default(),
    //     };
    //     stmts.visit_mut_children_with(&mut hoister)
    // }

    // Phase 2.
    stmts.visit_mut_children_with(self)
  }

  fn visit_mut_var_decl(&mut self, decl: &mut VarDecl) {
    // self.in_type = false;

    // let old_hoist = self.hoist;

    // self.hoist = VarDeclKind::Var == decl.kind;
    let ident_type = self.ident_type;
    self.ident_type = IdentType::Binding(decl.kind.clone());
    decl.decls.visit_mut_with(self);
    self.ident_type = ident_type;

    // self.hoist = old_hoist;
  }

  fn visit_mut_var_declarator(&mut self, decl: &mut VarDeclarator) {
    // order is important

    decl.name.visit_mut_with(self);

    let old_type = self.ident_type;
    self.ident_type = IdentType::Ref;
    decl.init.visit_mut_children_with(self);
    self.ident_type = old_type;
  }
}

// fn collect_ident_of_pat(pat: &Pat) -> Vec<&Ident> {
//   match pat {
//     // export const a = 1;
//     Pat::Ident(pat) => vec![&pat.id],
//     // export const [a] = [1]
//     Pat::Array(pat) => pat
//       .elems
//       .iter()
//       .flat_map(|pat| pat.as_ref().map_or(vec![], collect_ident_of_pat))
//       .collect(),
//     // TODO: export const { a } = { a: 1 }
//     // Pat::Object()
//     _ => vec![],
//   }
// }

pub struct Hoister<'me> {
  scope: &'me mut Scope,
}

impl<'me> Hoister<'me> {
  pub fn new(scope: &'me mut Scope) -> Self {
    assert_eq!(scope.kind, ScopeKind::Fn);
    Self { scope }
  }

  pub fn declare(&mut self, id: &mut Ident, _kind: VarDeclKind) {
    let cur_mark = self.scope.mark;
    let ctxt = SyntaxContext::empty().apply_mark(Mark::fresh(cur_mark));
    id.span.ctxt = ctxt;
    self.scope.declared_symbols.insert(id.sym.clone(), ctxt);
  }

  pub fn declare_pat(&mut self, pat: &mut Pat, kind: VarDeclKind) {
    collect_mut_ident_of_pat(pat)
      .into_iter()
      .for_each(|ident| self.declare(ident, kind));
  }
}

impl<'me> VisitMut for Hoister<'me> {
  fn visit_mut_var_decl(&mut self, node: &mut VarDecl) {
    node.decls.iter_mut().for_each(|declarator| {
      self.declare_pat(&mut declarator.name, node.kind.clone());
    });
  }
}
