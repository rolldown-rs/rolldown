use swc_ecma_ast::{Expr, ModuleDecl, ModuleItem, OptChainBase, OptChainExpr, PatOrExpr, Stmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideEffect {
  Todo,
  FnCall,
  VisitProp,
  VisitThis,
  NonTopLevel,
  VisitGlobalVar,
  Import,
}

fn detect_side_effect_of_expr(expr: &Expr) -> Option<SideEffect> {
  match expr {
    Expr::This(_) => Some(SideEffect::VisitThis),
    Expr::Array(array_lit) => array_lit.elems.iter().find_map(|expr_or_spread| {
      expr_or_spread
        .as_ref()
        .and_then(|exp| detect_side_effect_of_expr(exp.expr.as_ref()))
    }),
    Expr::Object(_) => Some(SideEffect::Todo),

    Expr::Fn(_) => None,

    Expr::Unary(_) => Some(SideEffect::Todo),

    // `++v`, `--v`, `v++`, `v--`
    Expr::Update(update_expr) => detect_side_effect_of_expr(update_expr.arg.as_ref()),

    Expr::Bin(bin_expr) => [bin_expr.left.as_ref(), bin_expr.right.as_ref()]
      .into_iter()
      .find_map(detect_side_effect_of_expr),

    Expr::Assign(assign_expr) => match &assign_expr.left {
      PatOrExpr::Expr(expr) => detect_side_effect_of_expr(expr.as_ref()),
      PatOrExpr::Pat(_) => Some(SideEffect::Todo),
    },
    Expr::Member(_) => Some(SideEffect::VisitProp),
    Expr::SuperProp(_) => Some(SideEffect::VisitProp),

    // true ? 'a' : 'b'
    Expr::Cond(cond_expr) => [
      cond_expr.test.as_ref(),
      cond_expr.cons.as_ref(),
      cond_expr.alt.as_ref(),
    ]
    .into_iter()
    .find_map(detect_side_effect_of_expr),

    Expr::Call(_) => Some(SideEffect::FnCall),
    // `new Cat()`
    Expr::New(_) => Some(SideEffect::FnCall),

    Expr::Seq(seq_expr) => seq_expr
      .exprs
      .iter()
      .find_map(|expr| detect_side_effect_of_expr(expr)),

    Expr::Ident(_) => None,

    Expr::Lit(_) => None,

    Expr::Tpl(tpl) => tpl
      .exprs
      .iter()
      .find_map(|expr| detect_side_effect_of_expr(expr)),

    Expr::TaggedTpl(_) => Some(SideEffect::FnCall),

    Expr::Arrow(_) => None,

    Expr::Class(_) => None,

    Expr::Yield(_) => Some(SideEffect::Todo),

    Expr::MetaProp(_) => Some(SideEffect::Todo),

    Expr::Await(_) => Some(SideEffect::Todo),

    Expr::Paren(paren_expr) => detect_side_effect_of_expr(paren_expr.expr.as_ref()),

    Expr::JSXMember(_) => Some(SideEffect::Todo),

    Expr::JSXNamespacedName(_) => Some(SideEffect::Todo),

    Expr::JSXEmpty(_) => Some(SideEffect::Todo),

    Expr::JSXElement(_) => Some(SideEffect::Todo),

    Expr::JSXFragment(_) => Some(SideEffect::Todo),

    Expr::TsTypeAssertion(_) => None,

    Expr::TsConstAssertion(_) => None,

    Expr::TsNonNull(_) => None,

    Expr::TsAs(_) => None,

    Expr::TsInstantiation(_) => None,

    Expr::PrivateName(_) => Some(SideEffect::Todo),

    Expr::OptChain(OptChainExpr {
      base: OptChainBase::Member(member),
      ..
    }) => detect_side_effect_of_expr(&Expr::Member(member.clone())),

    Expr::OptChain(OptChainExpr {
      base: OptChainBase::Call(_),
      ..
    }) => None,

    Expr::Invalid(_) => Some(SideEffect::Todo),
  }
}

// ESM environment
pub fn detect_side_effect(item: &ModuleItem) -> Option<SideEffect> {
  match item {
    ModuleItem::ModuleDecl(ModuleDecl::Import(_)) => Some(SideEffect::Import),
    ModuleItem::Stmt(stmt) => match stmt {
      // `{ }`
      Stmt::Block(_) => Some(SideEffect::NonTopLevel),
      // `;`
      Stmt::Empty(_) => None,
      // `debugger`
      Stmt::Debugger(_) => Some(SideEffect::Todo),
      // `with(foo) {}`
      Stmt::With(_) => Some(SideEffect::Todo),
      // `return`
      Stmt::Return(_) => Some(SideEffect::Todo),
      // s
      Stmt::Labeled(_) => Some(SideEffect::Todo),

      Stmt::Break(_) => Some(SideEffect::Todo),

      Stmt::Continue(_) => Some(SideEffect::Todo),

      Stmt::If(_) => Some(SideEffect::Todo),

      Stmt::Switch(_) => Some(SideEffect::Todo),

      Stmt::Throw(_) => Some(SideEffect::Todo),
      Stmt::Try(_) => Some(SideEffect::Todo),

      Stmt::While(_) => Some(SideEffect::Todo),

      Stmt::DoWhile(_) => Some(SideEffect::Todo),

      Stmt::For(_) => Some(SideEffect::Todo),

      Stmt::ForIn(_) => Some(SideEffect::Todo),

      Stmt::ForOf(_) => Some(SideEffect::Todo),
      Stmt::Decl(_) => None,
      Stmt::Expr(expr_stmt) => detect_side_effect_of_expr(expr_stmt.expr.as_ref()),
    },
    _ => None,
  }
}
