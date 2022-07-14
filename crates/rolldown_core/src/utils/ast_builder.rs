use ast::{Id, ModuleItem};
use hashbrown::{HashMap, HashSet};
use swc_atoms::JsWord;
use swc_common::{util::take::Take, Mark, DUMMY_SP};
use swc_ecma_utils::quote_ident;

pub fn gen_namespace_export_stmt(var_name: Id, exports: &HashMap<JsWord, Id>) -> ModuleItem {
    // use ast::{PropOrSpread, PropName, Prop, Expr, Lit, Null, Stmt, KeyValueProp, Decl};
    use ast::*;
    let mut key_values = exports
        .into_iter()
        .filter(|(name, _)| *name != "*")
        .collect::<Vec<_>>();
    key_values.sort_by(|a, b| a.0.cmp(b.0));
    let mut props = vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(quote_ident!("__proto__")),
        value: Box::new(Expr::Lit(Lit::Null(Null::dummy()))),
    })))];
    props.append(
        &mut key_values
            .into_iter()
            .map(|(name, id)| {
                PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                    key: {
                        if Ident::verify_symbol(name).is_ok() {
                            PropName::Ident(quote_ident!(DUMMY_SP.apply_mark(Mark::new()), name))
                        } else {
                            PropName::Str(name.clone().into())
                        }
                    },
                    value: Box::new(Expr::Ident(id.clone().into())),
                })))
            })
            .collect(),
    );
    ModuleItem::Stmt(Stmt::Decl(Decl::Var(VarDecl {
        span: DUMMY_SP,
        kind: VarDeclKind::Var,
        declare: false,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            definite: false,
            name: Pat::Ident(BindingIdent {
                type_ann: None,
                id: var_name.into(),
            }),
            init: Some(Box::new(Expr::Call(CallExpr {
                callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                    obj: Box::new(Expr::Ident(Ident {
                        sym: "Object".into(),
                        ..Ident::dummy()
                    })),
                    prop: MemberProp::Ident(Ident {
                        sym: "freeze".into(),
                        ..Ident::dummy()
                    }),
                    ..MemberExpr::dummy()
                }))),
                args: vec![ExprOrSpread {
                    expr: Box::new(Expr::Object(ObjectLit {
                        span: DUMMY_SP,
                        props,
                    })),
                    spread: None,
                }],
                ..CallExpr::dummy()
            }))),
        }],
    })))
}


pub fn gen_exports_stmt(exports: &HashMap<JsWord, Id>) -> ast::ModuleItem {
  use ast::{
      ExportNamedSpecifier, ExportSpecifier, Ident, ModuleDecl, ModuleExportName, NamedExport,
  };
  use swc_common::{Span, DUMMY_SP};
  let mut exports = exports.into_iter().collect::<Vec<_>>();
  exports.sort_by(|a, b| a.0.cmp(b.0));

  ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(NamedExport {
      span: Default::default(),
      specifiers: exports
          .into_iter()
          .filter(|(name, _)| name != &"*")
          .map(|(name, id)| {
              ExportSpecifier::Named(ExportNamedSpecifier {
                  span: Default::default(),
                  orig: ModuleExportName::Ident(ast::Ident {
                      sym: id.0.clone(),
                      span: Span {
                          ctxt: id.1,
                          ..DUMMY_SP
                      },
                      optional: false,
                  }),
                  exported: Some(ModuleExportName::Ident(Ident {
                      sym: name.clone(),
                      ..Ident::dummy()
                  })),
                  is_type_only: false,
              })
          })
          .collect::<Vec<_>>(),
      src: None,
      type_only: false,
      asserts: None,
  }))
}