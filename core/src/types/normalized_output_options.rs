pub struct NormalizedOutputOptions {
  // --- Options Rolldown doesn't need to be supported
// /** @deprecated Use the "renderDynamicImport" plugin hook instead. */
// dynamicImportFunction: string | undefined;

// amd: NormalizedAmdOptions;
// assetFileNames: string | ((chunkInfo: PreRenderedAsset) => string);
// banner: () => string | Promise<string>;
// chunkFileNames: string | ((chunkInfo: PreRenderedChunk) => string);
// compact: boolean;
// dir: string | undefined;
// entryFileNames: string | ((chunkInfo: PreRenderedChunk) => string);
// esModule: boolean;
// exports: 'default' | 'named' | 'none' | 'auto';
// extend: boolean;
// externalLiveBindings: boolean;
// file: string | undefined;
// footer: () => string | Promise<string>;
// format: InternalModuleFormat;
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
// /** @deprecated Use the "renderDynamicImport" plugin hook instead. */
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
}

impl Default for NormalizedOutputOptions {
  fn default() -> Self {
    Self {}
  }
}
