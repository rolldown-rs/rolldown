use std::collections::HashMap;

use log::debug;
use once_cell::sync::Lazy;
use swc_common::{sync::Lrc, SourceMap};

use crate::analyse::DynImportDesc;
use crate::types::{ModOrExt, ResolveIdResult, ResolvedId, UnresolvedModule};
use crate::utils;
use crate::utils::resolve_id::resolve_id;
use crate::utils::transform::transform;
use crate::{external_module::ExternalModule, module::Module};
use crate::{
  types::{shared, Shared},
  utils::plugin_driver::PluginDriver,
  // GraphError,
};

pub(crate) static SOURCE_MAP: Lazy<Lrc<SourceMap>> = Lazy::new(Default::default);

#[derive(Clone)]
pub struct ModuleLoader {
  pub modules_by_id: HashMap<String, ModOrExt>,
  plugin_driver: Shared<PluginDriver>,
}

impl ModuleLoader {
  pub fn new(plugin_driver: Shared<PluginDriver>) -> Shared<Self> {
    shared(Self {
      modules_by_id: HashMap::default(),
      plugin_driver,
    })
  }

  fn add_module_source(&self, id: &str, _importer: Option<&str>, module: &mut Module) {
    debug!("add_module_source of id: {}", id);
    let source = self
      .plugin_driver
      .borrow()
      .load(id)
      .unwrap_or(std::fs::read_to_string(id).unwrap());
    // hook `load` was called

    module.update_options();

    let transformed = transform(source, module, &self.plugin_driver.borrow());
    // hook `transform` was called

    module.set_source(transformed)
  }

  pub(crate) fn fetch_module(
    &mut self,
    resolved_id: &ResolvedId,
    importer: Option<&str>,
    is_entry: bool,
  ) -> Shared<Module> {
    let id = &resolved_id.id;
    if let Some(ModOrExt::Mod(m)) = self.modules_by_id.get(id) {
      m.clone()
    } else {
      let module = Module::new(id.into(), is_entry);
      self.modules_by_id.insert(id.into(), module.clone().into());
      self.add_module_source(id, importer, &mut module.borrow_mut());
      let resolve_static_dependency = self.get_resolve_static_dependency(&mut module.borrow_mut());
      let mut resolve_dynamic_import = self.get_resolve_dynamic_import(&mut module.borrow_mut());

      // After resolving dependencies of the module. Rollup think the module is fullly parsed.
      // So, we call `moduleParsed` hook.

      self.plugin_driver.borrow().module_parsed();

      self.fetch_module_dependencies(
        &mut module.borrow_mut(),
        &resolve_static_dependency,
        &mut resolve_dynamic_import,
      );

      module
    }
  }

  fn fetch_module_dependencies(
    &mut self,
    module: &mut Module,
    static_dependency: &[(String, ResolvedId)],
    dynamic_dependency: &mut [(DynImportDesc, Option<ResolvedId>)],
  ) {
    self.fetch_static_dependencies(module, static_dependency);
    self.fetch_dynamic_dependencies(module, dynamic_dependency);
    module.link_imports(self);
  }

  fn fetch_static_dependencies(
    &mut self,
    module: &mut Module,
    dependencies: &[(String, ResolvedId)],
  ) {
    dependencies
      .iter()
      .map(|(source, resolved_id)| self.fetch_resolved_dependency(source, &module.id, &resolved_id))
      .for_each(|dep| {
        dep.add_importers(module.id.clone());
        module.dependencies.push(dep);
      });
  }

  fn fetch_dynamic_dependencies(
    &mut self,
    module: &mut Module,
    resolve_dynamic_import: &mut [(DynImportDesc, Option<ResolvedId>)],
  ) {
    resolve_dynamic_import
      .iter_mut()
      .flat_map(|(dynamic_import, resolved_id)| {
        if let Some(resolved_id) = resolved_id {
          let dep = self.fetch_resolved_dependency(
            &utils::path::relative_id(resolved_id.id.clone().into())
              .to_str()
              .unwrap(),
            &module.id,
            resolved_id,
          );
          dynamic_import.resolution = Some(dep.clone());
          Some(dep)
        } else {
          None
        }
      })
      .for_each(|dep| {
        dep.add_dynamic_importers(module.id.clone());
        module.dynamic_dependencies.push(dep);
      });
  }

  fn fetch_resolved_dependency(
    &mut self,
    source: &str,
    importer: &str,
    resolved_id: &ResolvedId,
  ) -> ModOrExt {
    debug!("fetch_resolved_dependency for {:#?}", resolved_id);
    if resolved_id.external {
      let module = self
        .modules_by_id
        .entry(resolved_id.id.clone())
        .or_insert(ExternalModule::new(resolved_id.id.clone()).into());
      if module.is_mod() {
        panic!("errInternalIdCannotBeExternal: {}", source)
      }
      module.clone()
    } else {
      self.fetch_module(resolved_id, Some(importer), false).into()
    }
  }

  fn get_resolve_static_dependency(&mut self, module: &mut Module) -> Vec<(String, ResolvedId)> {
    module
      .sources
      .iter()
      .map(|source| {
        let resolved_id;
        if let Some(resolved) = module.resolved_ids.get(source) {
          resolved_id = resolved.clone();
        } else {
          resolved_id = self.resolve_id(source, Some(&module.id), false).unwrap();
          module
            .resolved_ids
            .insert(source.clone(), resolved_id.clone());
        };

        (source.clone(), resolved_id)
      })
      .collect()
  }

  fn get_resolve_dynamic_import(
    &self,
    module: &mut Module,
  ) -> Vec<(DynImportDesc, Option<ResolvedId>)> {
    let unsafe_module_mut_p = unsafe {
      let p = module as *mut Module;
      p.as_mut().unwrap()
    };
    module
      .dynamic_imports
      .iter_mut()
      .map(|dynamic_import| {
        let resolved_id =
          self.resolve_dynamic_import(unsafe_module_mut_p, &dynamic_import.argument, &module.id);
        // if let Some(resolved_id) = &resolved_id {
        //   dynamic_import.id = Some(resolved_id.id.clone());
        // }
        (dynamic_import.clone(), resolved_id.clone())
      })
      .collect()
  }

  fn resolve_dynamic_import(
    &self,
    module: &mut Module,
    specifier: &str,
    importer: &str,
  ) -> ResolveIdResult {
    let resolution = self
      .plugin_driver
      .borrow()
      .resolve_dynamic_import(specifier, importer);

    if let Some(resolution) = resolution {
      Some(resolution)
    } else {
      if let Some(resolved) = module.resolved_ids.get(specifier) {
        Some(resolved.clone())
      } else {
        let resolved =
          module
            .resolved_ids
            .entry(specifier.to_owned())
            .or_insert(self.handle_resolve_id(
              self.resolve_id(specifier, Some(importer), false),
              specifier,
              importer,
            ));
        Some(resolved.clone())
      }
    }
  }

  fn handle_resolve_id(
    &self,
    resolved_id: ResolveIdResult,
    source: &str,
    _importer: &str,
  ) -> ResolvedId {
    if let None = resolved_id {
      ResolvedId::new(source.to_owned(), true)
    } else {
      resolved_id.unwrap()
    }
  }

  fn external(&self, _source: &str, _importer: Option<&str>, _is_resolved: bool) -> bool {
    false
  }

  fn resolve_id(
    &self,
    source: &str,
    importer: Option<&str>,
    _is_entry: bool,
  ) -> Option<ResolvedId> {
    let is_external = self.external(source, importer, false);
    if is_external {
      None
    } else {
      let id = resolve_id(source, importer, false, &self.plugin_driver.borrow());
      id.map(|part| ResolvedId::new(part.id, part.external))
    }
  }

  pub fn add_entry_modules(
    &mut self,
    entries: &[UnresolvedModule],
    _is_user_defined: bool,
  ) -> Vec<Shared<Module>> {
    let entry_modules = entries
      .iter()
      .map(|unresolved| {
        self.load_entry_module(
          &unresolved.id,
          true,
          unresolved.importer.as_ref().map(|s| s.as_str()),
        )
      })
      .collect::<Vec<Shared<Module>>>();

    entry_modules.iter().for_each(|_entry_module| {
      // entry_module.borrow_mut().is
    });

    entry_modules
  }

  pub fn load_entry_module(
    &mut self,
    unresolved_id: &str,
    is_entry: bool,
    importer: Option<&str>,
  ) -> Shared<Module> {
    debug!("load_entry_module for unresolved_id {}", unresolved_id);
    let resolve_id_result =
      resolve_id(unresolved_id, importer, false, &self.plugin_driver.borrow());
    // hook `resoveId` was called

    if let Some(resolve_id_result) = resolve_id_result {
      self.fetch_module(&resolve_id_result, importer, is_entry)
    } else {
      panic!("resolve_id_result is None")
    }
  }
}
