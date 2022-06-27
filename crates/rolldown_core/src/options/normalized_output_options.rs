#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InternalModuleFormat {
  ES,
  CJS,
  AMD,
  UMD,
}

#[derive(Debug)]
pub struct NormalizedOutputOptions {
  // --- Options Rolldown doesn't need to be supported
  // /** @deprecated Use the "renderDynamicImport" plugin hook instead. */
  // dynamicImportFunction: string | undefined;

  // amd: NormalizedAmdOptions;
  // assetFileNames: string | ((chunkInfo: PreRenderedAsset) => string);
  // banner: () => string | Promise<string>;
  // chunkFileNames: string | ((chunkInfo: PreRenderedChunk) => string);
  // compact: boolean;
  pub dir: Option<String>,
  pub entry_file_names: String, // | ((chunkInfo: PreRenderedChunk) => string)
  // esModule: boolean;
  // exports: 'default' | 'named' | 'none' | 'auto';
  // extend: boolean;
  // externalLiveBindings: boolean;
  pub file: Option<String>,
  // footer: () => string | Promise<string>;
  pub format: InternalModuleFormat,
  // freeze: boolean;
  // generatedCode: NormalizedGeneratedCodeOptions;
  // globals: GlobalsOption;
  // hoistTransitiveImports: boolean;
  // indent: true | string;
  // inlineDynamicImports: boolean;
  // interop: GetInterop;
  // intro: () => string | Promise<string>;
  // manualChunks: ManualChunksOption;
  // minifyInternalExports: boolean;
  // name: string | undefined;
  // namespaceToStringTag: boolean;
  // noConflict: boolean;
  // outro: () => string | Promise<string>;
  // paths: OptionsPaths;
  // plugins: OutputPlugin[];
  // preferConst: boolean;
  // preserveModules: boolean;
  // preserveModulesRoot: string | undefined;
  // sanitizeFileName: (fileName: string) => string;
  // sourcemap: boolean | 'inline' | 'hidden';
  // sourcemapExcludeSources: boolean;
  // sourcemapFile: string | undefined;
  // sourcemapPathTransform: SourcemapPathTransformOption | undefined;
  // strict: boolean;
  // systemNullSetters: boolean;
  // validate: boolean;
  // --- Enhanced options
  pub minify: bool,
}

impl Default for NormalizedOutputOptions {
  fn default() -> Self {
    Self {
      format: InternalModuleFormat::ES,
      file: Default::default(),
      dir: Default::default(),
      minify: Default::default(),
      entry_file_names: "[name].js".to_string(),
    }
  }
}
