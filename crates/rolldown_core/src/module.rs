use std::fmt::Debug;

use ast::{Id, ModuleItem};
use hashbrown::{HashMap, HashSet};
use linked_hash_set::LinkedHashSet;
use swc_atoms::JsWord;
use swc_common::{util::take::Take, Mark, DUMMY_SP};
use swc_ecma_codegen::text_writer::JsWriter;
use swc_ecma_utils::quote_ident;

use crate::{
    get_swc_compiler, ufriend::UFriend, LocalExports, MergedExports, ModuleById, ResolvedId,
    SideEffect, Specifier, SpecifierId,
};

pub struct Module {
    pub exec_order: usize,
    pub id: JsWord,
    pub dependencies: LinkedHashSet<JsWord>,
    pub dyn_dependencies: HashSet<JsWord>,
    // source: String,
    pub ast: ast::Program,
    pub top_level_mark: Mark,
    pub imports: HashMap<JsWord, HashSet<SpecifierId>>,
    pub re_exports: HashMap<JsWord, HashSet<Specifier>>,
    pub local_exports: LocalExports,
    pub merged_exports: MergedExports,
    pub side_effect: Option<SideEffect>,
    pub resolved_module_ids: HashMap<JsWord, ResolvedId>,
    pub declared_ids: HashSet<Id>,
    pub included: bool,
    pub used_exported_id: HashSet<Id>,
    pub suggested_names: HashMap<JsWord, JsWord>,
    pub is_user_defined_entry: bool,
}

impl Module {
    pub fn suggest_name(&mut self, name: JsWord, suggested: JsWord) {
        self.suggested_names.insert(name, suggested);
    }

    pub fn depended_modules<'a>(&self, module_graph: &'a ModuleById) -> Vec<&'a Module> {
        self.dependencies
            .iter()
            .map(|unresolved_id| self.resolved_module_ids.get(unresolved_id).unwrap())
            .filter_map(|dep| module_graph.get(&dep.id))
            .collect()
    }

    pub fn dynamic_depended_modules<'a>(&self, module_graph: &'a ModuleById) -> Vec<&'a Module> {
        self.dyn_dependencies
            .iter()
            .map(|unresolved_id| self.resolved_module_ids.get(unresolved_id).unwrap())
            .filter_map(|dep| module_graph.get(&dep.id))
            .collect()
    }

    fn gen_namespace_export(&self, name_id: Id) -> ast::ModuleItem {
        // use ast::{PropOrSpread, PropName, Prop, Expr, Lit, Null, Stmt, KeyValueProp, Decl};
        use ast::*;
        let mut key_values = self
            .merged_exports
            .iter()
            .filter(|(name, _)| {
              *name != "*"
            })
            .collect::<Vec<_>>();
        key_values.sort_by(|a, b| a.0.cmp(b.0));
        let mut props = vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
            key: PropName::Str("__proto__".into()),
            value: Box::new(Expr::Lit(Lit::Null(Null::dummy()))),
        })))];
        props.append(
            &mut key_values
                .into_iter()
                .map(|(name, id)| {
                    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                        key: PropName::Str(name.to_string().into()),
                        value: Box::new(Expr::Ident(id.clone().into())),
                    })))
                })
                .collect(),
        );
        ModuleItem::Stmt(Stmt::Decl(Decl::Var(VarDecl {
            span: DUMMY_SP,
            kind: VarDeclKind::Const,
            declare: false,
            decls: vec![VarDeclarator {
                span: DUMMY_SP,
                definite: false,
                name: Pat::Ident(BindingIdent {
                    type_ann: None,
                    id: name_id.into(),
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

    pub fn mark_used_id(&mut self, name: &JsWord, id: &Id) {
        if name == "*" && !self.merged_exports.contains_key(&"*".into()) {
            let namespace_export = get_swc_compiler().run(|| {
                self.merged_exports.insert(
                    "*".into(),
                    quote_ident!(DUMMY_SP.apply_mark(Mark::new()), "*").to_id(),
                );
                self.gen_namespace_export(
                    quote_ident!(DUMMY_SP.apply_mark(Mark::new()), "*").to_id(),
                )
            });
            self.ast
                .as_mut_module()
                .unwrap()
                .body
                .push(namespace_export)
        };
        let local_id = self
            .merged_exports
            .get(name)
            .unwrap_or_else(|| panic!("fail to get id {:?} in {:?}", name, self.id))
            .clone();
        self.used_exported_id.insert(local_id.clone());
    }

    pub fn unused_ids(&self) -> HashSet<Id> {
        self.merged_exports
            .iter()
            .filter_map(|(name, id)| {
                if self.used_exported_id.contains(id) {
                    None
                } else {
                    Some(id.clone())
                }
            })
            .collect()
    }

    pub fn generate_exports(&mut self) {
        if !self.merged_exports.is_empty() {
            let exports = self.gen_export();
            self.ast.as_mut_module().map(|ast| ast.body.push(exports));
        }
    }

    pub fn render(&self) -> String {
        let mut output = Vec::new();

        let mut emitter = swc_ecma_codegen::Emitter {
            cfg: Default::default(),
            cm: get_swc_compiler().cm.clone(),
            comments: None,
            wr: Box::new(JsWriter::new(
                get_swc_compiler().cm.clone(),
                "\n",
                &mut output,
                None,
            )),
        };

        emitter.emit_program(&self.ast).unwrap();
        String::from_utf8(output).unwrap()
    }

    pub fn gen_export(&self) -> ast::ModuleItem {
        use ast::{
            ExportNamedSpecifier, ExportSpecifier, Ident, ModuleDecl, ModuleExportName, NamedExport,
        };
        use swc_common::{Span, DUMMY_SP};
        let mut exports = self.merged_exports.iter().collect::<Vec<_>>();
        exports.sort_by(|a, b| a.0.cmp(b.0));

        ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(NamedExport {
            span: Default::default(),
            specifiers: exports
                .into_iter()
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
}

impl Debug for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Module")
            .field("exec_order", &self.exec_order)
            .field("id", &self.id)
            .field("dependencies", &self.dependencies)
            .field("dyn_dependencies", &self.dyn_dependencies)
            .field("imports", &self.imports)
            .field("re_exports", &self.re_exports)
            .field("local_exports", &self.local_exports)
            .field("merged_exports", &self.merged_exports)
            .field("side_effect", &self.side_effect)
            .field("resolved_module_ids", &self.resolved_module_ids)
            .field("ast", &"...")
            .field("included", &self.included)
            .field("used_ids", &self.used_exported_id)
            .field("unused_ids", &self.unused_ids())
            .field("declared_ids", &self.declared_ids)
            .finish()
    }
}
