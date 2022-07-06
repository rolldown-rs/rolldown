use crate::{TreeShakeExportRemover, Module};


use ast::Id;
use hashbrown::HashSet;
use swc_common::{Mark, chain};
use swc_ecma_transforms_optimization::simplify;
use swc_ecma_visit::{FoldWith};

use crate::get_swc_compiler;

pub fn shake(unused_ids: &HashSet<Id>, ast: ast::Module, unresolved_mark: Mark) -> ast::Module {
    get_swc_compiler().run(|| {
        let export_remover = TreeShakeExportRemover {
            unused_ids,
        };
        let mut pass = chain!(export_remover, simplify::simplifier(
          unresolved_mark,
          Default::default(),
      ));
        ast.fold_with(&mut pass)
    })
}
