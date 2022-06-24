use hashbrown::HashSet;
use linked_hash_set::LinkedHashSet;
use swc_atoms::JsWord;
use swc_ecma_ast::{CallExpr, Callee, ExportSpecifier, Expr, ExprOrSpread, Lit, ModuleDecl};
use swc_ecma_visit::{noop_visit_mut_type, VisitMut, VisitMutWith};

#[derive(Default)]
pub struct DependencyScanner {
  pub dependencies: LinkedHashSet<JsWord>,
  pub dyn_dependencies: HashSet<JsWord>,
  // pub dyn_dependencies: HashSet<DynImportDesc>,
}

impl DependencyScanner {
  fn add_dependency(&mut self, specifier: JsWord) {
    self.dependencies.insert_if_absent(specifier);
  }

  // 
  
  fn add_dynamic_import(&mut self, node: &CallExpr) {
    if let Callee::Import(_) = node.callee {
      if let Some(dyn_imported) = node.args.get(0) {
        if dyn_imported.spread.is_none() {
          if let Expr::Lit(Lit::Str(imported)) = dyn_imported.expr.as_ref() {
            self.dyn_dependencies.insert(imported.value.clone());
          }
        }
      }
    }
  }

  fn add_import(&mut self, module_decl: &mut ModuleDecl) {
    if let ModuleDecl::Import(import_decl) = module_decl {
      let source = import_decl.src.value.clone();
      self.add_dependency(source);
    }
  }

  fn add_export(&mut self, module_decl: &ModuleDecl) -> Result<(), anyhow::Error> {
    match module_decl {
      ModuleDecl::ExportNamed(node) => {
        node.specifiers.iter().for_each(|specifier| {
          match specifier {
            ExportSpecifier::Named(_s) => {
              if let Some(source_node) = &node.src {
                // export { name } from './other'
                let source = source_node.value.clone();
                self.add_dependency(source);
              }
            }
            ExportSpecifier::Namespace(_s) => {
              // export * as name from './other'
              let source = node.src.as_ref().map(|str| str.value.clone()).unwrap();
              self.add_dependency(source);
            }
            ExportSpecifier::Default(_) => {
              // export v from 'mod';
              // Rollup doesn't support it.
            }
          };
        });
      }
      ModuleDecl::ExportAll(node) => {
        // export * from './other'
        self.add_dependency(node.src.value.clone());
      }
      _ => {}
    }
    Ok(())
  }
}

impl VisitMut for DependencyScanner {
  noop_visit_mut_type!();

  fn visit_mut_module_decl(&mut self, node: &mut ModuleDecl) {
    self.add_import(node);
    if let Err(e) = self.add_export(node) {
      eprintln!("{}", e);
    }
    node.visit_mut_children_with(self);
  }
  fn visit_mut_call_expr(&mut self, node: &mut CallExpr) {
    self.add_dynamic_import(node);
    node.visit_mut_children_with(self);
  }
}
