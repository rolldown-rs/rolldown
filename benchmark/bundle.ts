import { promises as fs } from 'fs'
import { join } from 'path'

import { blue } from 'colorette'
import { build } from 'esbuild'
import { rollup } from 'rollup'

import { rolldown } from '../node'

const LODASH_ENTRY = require.resolve('lodash-es')
const THREE_JS_ENTRY = join(__dirname, 'three.js', 'src', 'Three.js')

async function bench(entry: string, entryName: string) {
  const beforeEsbuild = process.hrtime.bigint()
  const name = blue(entryName)
  const {
    outputFiles: [{ text }],
  } = await build({
    entryPoints: [entry],
    bundle: true,
    treeShaking: true,
    sourcemap: true,
    minify: false,
    splitting: false,
    write: false,
    target: 'esnext',
  })
  const esbuildDuration = process.hrtime.bigint() - beforeEsbuild
  console.info(`esbuild [${name}]: `, Number(esbuildDuration / BigInt(1e6)).toFixed(2), 'ms')
  await fs.writeFile(join(__dirname, `esbuild-${entryName}.js`), text)
  const beforeRolldown = process.hrtime.bigint()
  const code = await rolldown(entry)
  const rolldownDuration = process.hrtime.bigint() - beforeRolldown
  console.info(`rolldown: [${name}]`, Number(rolldownDuration / BigInt(1e6)).toFixed(2), 'ms')
  await fs.writeFile(join(__dirname, `rolldown-${entryName}.js`), code)
  const beforeRollup = process.hrtime.bigint()
  await rollup({
    input: entry,
    cache: false,
    treeshake: true,
  })
  const rollupDuration = process.hrtime.bigint() - beforeRollup
  console.info(`rollup: [${name}]`, Number(rollupDuration / BigInt(1e6)).toFixed(2), 'ms')
}

bench(LODASH_ENTRY, 'lodash-es')
  .then(() => bench(THREE_JS_ENTRY, 'three.js'))
  .catch((e) => {
    console.error(e)
    throw e
  })
