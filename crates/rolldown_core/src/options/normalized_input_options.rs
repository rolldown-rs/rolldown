use hashbrown::HashMap;

// (source: &str, importer: Option<&str>, is_resolved: bool)
pub type IsExternal = Box<dyn Fn(&str, Option<&str>, bool) -> bool>;

// type ModuleContext = Box<dyn Fn(&str) -> &str>;

// type EntryAlias = String;

#[derive(Default, Debug)]
pub struct NormalizedInputOptions {
    // --- Options that Rolldown doesn't need to be supported
    // acorn: Record<string, unknown>;
    // acornInjectPlugins: (() => unknown)[];
    // experimentalCacheExpiry: number;
    // /** @deprecated Use the "inlineDynamicImports" output option instead. */
    // inlineDynamicImports: boolean | undefined;
    // /** @deprecated Use the "manualChunks" output option instead. */
    // manualChunks: ManualChunksOption | undefined;
    // /** @deprecated Use the "preserveModules" output option instead. */
    // preserveModules: boolean | undefined;
    // When this flag is enabled, Rollup will throw an error instead of showing a warning when a deprecated feature is used. Furthermore, features that are marked to receive a deprecation warning with the next major version will also throw an error when used.
    // strictDeprecations: boolean;

    // --- Options that Rolldown might need to supported
    // cache: false | undefined | RollupCache;
    // makeAbsoluteExternalsRelative: boolean | 'ifRelativeSource';
    // maxParallelFileReads: number;
    // onwarn: WarningHandler;
    // perf: boolean;
    // preserveEntrySignatures: PreserveEntrySignaturesOption;
    // shimMissingExports: boolean;
    // pub module_context: ModuleContext,

    // --- Options that Rolldown must need to be supported
    pub treeshake: bool,
    // pub treeshake: bool | NormalizedTreeshakingOptions;
    // pub plugins: Vec<Box<dyn Plugin>>,
    // By default, the context of a module – i.e., the value of this at the top level – is undefined. In rare cases you might need to change this to something else, like 'window'.
    // pub context: Option<String>,
    // pub external: IsExternal,
    // (alias: Option<String>, path: String)
    pub input: HashMap<String, InputItem>,
    // pub preserve_symlinks: bool,

    // --- extra
    pub root: String,
}

#[derive(Debug)]
pub struct InputItem {
    pub path: String,
}

impl From<String> for InputItem {
    fn from(s: String) -> Self {
        Self { path: s }
    }
}
