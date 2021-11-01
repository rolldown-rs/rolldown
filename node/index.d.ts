export interface Options {
  sourcemap?: boolean
}

export function rolldown(entry: string, options?: Options): Promise<{ code: Buffer; map: Buffer | null }>
