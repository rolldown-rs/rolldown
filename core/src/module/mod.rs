use std::{
  collections::{HashMap, HashSet},
  hash::Hash,
};

use crate::module_loader::{ModuleLoader, SOURCE_MAP};

use self::analyse::{
  get_module_info_from_ast, parse_file, DynImportDesc, ExportDesc, ImportDesc, ReExportDesc,
};
use crate::types::{shared, ModOrExt, ResolvedId, Shared};
pub mod analyse;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
  pub original_code: Option<String>,
  pub is_entry: bool,
  pub id: String,
  pub imports: HashMap<String, ImportDesc>,
  pub exports: HashMap<String, ExportDesc>,
  pub dynamic_imports: Vec<DynImportDesc>,
  // Named re_export. sush as `export { foo } from ...` or `export * as foo from '...'`
  pub re_exports: HashMap<String, ReExportDesc>,
  // Just re-export. sush as `export * from ...`
  pub export_all_sources: HashSet<String>,
  pub exports_all: HashMap<String, String>,
  // id of imported modules
  pub sources: HashSet<String>,
  pub resolved_ids: HashMap<String, ResolvedId>,
  // id of importers
  pub importers: HashSet<String>,
  pub export_all_modules: Vec<ModOrExt>,
  pub is_user_defined_entry_point: bool,
  // FIXME: we should use HashSet for this
  pub dependencies: HashSet<ModOrExt>,
  // FIXME: we should use HashSet for this
  pub dynamic_dependencies: HashSet<ModOrExt>,
  pub dynamic_importers: HashSet<String>,
}

impl Module {
  pub fn new(id: String, is_entry: bool) -> Shared<Self> {
    shared(Module {
      original_code: None,
      id,
      is_entry,
      imports: HashMap::default(),
      exports: HashMap::default(),
      re_exports: HashMap::default(),
      dynamic_imports: Vec::default(),
      export_all_sources: HashSet::default(),
      exports_all: HashMap::default(),
      sources: HashSet::default(),
      resolved_ids: HashMap::default(),
      is_user_defined_entry_point: false,
      dependencies: HashSet::default(),
      dynamic_dependencies: HashSet::default(),
      importers: HashSet::default(),
      dynamic_importers: HashSet::default(),
      export_all_modules: Vec::default(),
      // definitions,
      // modifications,
      // defined: RwLock::new(HashSet::default()),
      // suggested_names: RwLock::new(HashMap::default()),
    })
  }
}

impl Hash for Module {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    state.write(&self.id.as_bytes());
  }
}

impl Module {
  pub fn set_source(&mut self, source: String) {
    let ast = parse_file(source, self.id.clone(), &SOURCE_MAP).unwrap();
    let module_info = get_module_info_from_ast(&ast, self.id.clone());

    self.imports = module_info.imports;
    self.exports = module_info.exports;
    self.export_all_sources = module_info.export_all_sources;
    self.dynamic_imports = module_info.dyn_imports;
    self.sources = module_info.sources;
  }

  pub fn update_options(&self) {}

  pub fn link_imports(&mut self, module_loader: &ModuleLoader) {
    self.add_modules_to_import_descriptions(module_loader);
    self.add_modules_to_re_export_descriptions(module_loader);

    self.exports.keys().for_each(|name| {
      if name != "default" {
        self.exports_all.insert(name.clone(), self.id.clone());
      }

      let mut external_modules = vec![];
      self.export_all_sources.iter().for_each(|source| {
        let module_id = &self.resolved_ids.get(source).unwrap().id;
        let module = module_loader.modules_by_id.get(module_id).unwrap();
        match module {
          ModOrExt::Ext(module) => {
            external_modules.push(module.clone());
          }
          ModOrExt::Mod(module) => {
            self.export_all_modules.push(module.clone().into());
            module.borrow().exports_all.keys().for_each(|name| {
              if self.exports_all.contains_key(name) {
                panic!("NamespaceConflict")
              }
              self.exports_all.insert(name.clone(), name.clone());
            })
          }
        }
      });
      self.export_all_modules.append(
        &mut external_modules
          .iter()
          .map(|ext| ext.clone().into())
          .collect(),
      );
    })
  }

  fn add_modules_to_import_descriptions(&mut self, module_loader: &ModuleLoader) {
    self.imports.values_mut().for_each(|specifier| {
      let id = &self.resolved_ids.get(&specifier.source).unwrap().id;
      let module = module_loader.modules_by_id.get(id).unwrap();
      specifier.module.replace(module.clone());
    });
  }

  fn add_modules_to_re_export_descriptions(&mut self, module_loader: &ModuleLoader) {
    self.re_exports.values_mut().for_each(|specifier| {
      let id = &self.resolved_ids.get(&specifier.source).unwrap().id;
      let module = module_loader.modules_by_id.get(id).unwrap();
      specifier.module.replace(module.clone());
    });
  }

  pub fn get_dependencies_to_be_included(&self) -> Vec<ModOrExt> {
    let mut relevant_dependencies: HashSet<ModOrExt> = HashSet::new();
		let mut necessary_dependencies: HashSet<ModOrExt> = HashSet::new();
		let mut always_checked_dependencies: HashSet<Module> = HashSet::new();
    self.dependencies.clone().into_iter().collect()
  }
}
