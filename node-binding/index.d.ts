export interface Options {
  sourcemap?: boolean
}

export function rolldown(entry: string, options?: Options): Promise<string>
export type InputOption = string | string[] | { [entryAlias: string]: string }
export interface InputOptions {
  // --- Options that Rolldown doesn't need to be supported
  // acornInjectPlugins?: (() => unknown)[] | (() => unknown)
  // acorn?: Record<string, unknown>
  // --- Options that Rolldown might need to be supported
  // cache?: false | RollupCache
  // context?: string
  // experimentalCacheExpiry?: number
  // external?: ExternalOption
  // /** @deprecated Use the "inlineDynamicImports" output option instead. */
  // inlineDynamicImports?: boolean
  // makeAbsoluteExternalsRelative?: boolean | 'ifRelativeSource'
  // /** @deprecated Use the "manualChunks" output option instead. */
  // manualChunks?: ManualChunksOption
  // maxParallelFileReads?: number
  // moduleContext?: ((id: string) => string | null | undefined) | { [id: string]: string }
  // onwarn?: WarningHandlerWithDefault
  // perf?: boolean
  // plugins?: (Plugin | null | false | undefined)[]
  // preserveEntrySignatures?: PreserveEntrySignaturesOption
  // /** @deprecated Use the "preserveModules" output option instead. */
  // preserveModules?: boolean
  // preserveSymlinks?: boolean
  // shimMissingExports?: boolean
  // strictDeprecations?: boolean
  // treeshake?: boolean | TreeshakingPreset | TreeshakingOptions
  // watch?: WatcherOptions | false
  // --- Options that Rolldown need to be supported
  input: InputOption
}
