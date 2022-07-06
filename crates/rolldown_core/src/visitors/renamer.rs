use ast::{ExportNamedSpecifier, Id, Ident, ObjectLit, PropName};
use hashbrown::HashMap;
use swc_atoms::JsWord;
use swc_common::{util::take::Take, SyntaxContext, DUMMY_SP};
use swc_ecma_visit::{VisitMut, VisitMutWith};

use crate::ufriend::UFriend;

#[derive(Debug)]
pub struct Renamer<'a> {
    pub uf: &'a UFriend<Id>,
    pub rename_map: &'a HashMap<Id, JsWord>,
}

impl<'a> Renamer<'a> {
    fn need_to_rename(&self, ident: &Ident) -> Option<&'a JsWord> {
        let root_id = self.uf.find_root(&ident.to_id())?;
        self.rename_map.get(&root_id)
    }

    fn rename(&self, ident: &mut Ident) -> Option<()> {
        let name = self.need_to_rename(ident)?;
        *ident = Ident::new(name.clone(), DUMMY_SP);
        Some(())
    }
}

impl<'a> VisitMut for Renamer<'a> {
    fn visit_mut_ident(&mut self, ident: &mut Ident) {
        self.rename(ident);
    }

    fn visit_mut_object_lit(&mut self, node: &mut ObjectLit) {
        node.props.iter_mut().for_each(|prop_or_spread| {
            if let ast::PropOrSpread::Prop(prop) = prop_or_spread {
                if prop.is_shorthand() {
                    if let ast::Prop::Shorthand(prop_key_ident) = prop.as_mut() {
                        let is_need_expanded = self
                            .need_to_rename(prop_key_ident)
                            .map_or(false, |name| name != &prop_key_ident.sym);
                        if is_need_expanded {
                            let mut key = prop_key_ident.clone();
                            key.span.ctxt = SyntaxContext::empty();
                            let replacement = Box::new(ast::Prop::KeyValue(ast::KeyValueProp {
                                key: ast::PropName::Ident(key),
                                value: Box::new(ast::Expr::Ident(prop_key_ident.clone())),
                            }));
                            *prop = replacement;
                        }
                    }
                }
            }
        });
        node.visit_mut_children_with(self);
        node.props.iter_mut().for_each(|prop_or_spread| {
            if let ast::PropOrSpread::Prop(prop) = prop_or_spread {
                if let ast::Prop::KeyValue(key_value) = prop.as_mut() {
                    if let PropName::Ident(prop_ident) = &mut key_value.key {
                        if let box ast::Expr::Ident(value_ident) = &mut key_value.value {
                            let could_be_shorthand = value_ident.sym == prop_ident.sym;
                            if could_be_shorthand {
                                let replacement =
                                    Box::new(ast::Prop::Shorthand(value_ident.take()));
                                *prop = replacement;
                            }
                        }
                    }
                }
            }
        });
    }

    fn visit_mut_export_named_specifier(&mut self, node: &mut ExportNamedSpecifier) {
        node.visit_mut_children_with(self);
        if let Some(ast::ModuleExportName::Ident(expr)) = &node.exported {
            if let ast::ModuleExportName::Ident(orig) = &node.orig {
                if expr.sym == orig.sym {
                    node.exported = None
                }
            }
        }
    }

    fn visit_mut_member_expr(&mut self, node: &mut ast::MemberExpr) {
        // For a MemberExpr, AKA `a.b`, we only need to rename `a`;
        node.obj.visit_mut_with(self);
        if node.prop.is_computed() {
            // Handle `a[b]`
            node.prop.visit_mut_with(self);
        }
    }

    // TODO: There are more AST nodes we could skip for Renamer. Just like `visit_mut_member_expr`.
}
