import { join } from 'path'

import chalk from 'chalk'
import { build } from 'esbuild'
import { rollup } from 'rollup'

import { rolldown } from '../packages/node'

const LODASH_ENTRY = require.resolve('lodash-es')
const THREE_JS_ENTRY = join(__dirname, 'three.js', 'src', 'Three.js')

async function bench(entry: string, entryName: string) {
  const beforeEsbuild = process.hrtime.bigint()
  const name = chalk.blue(entryName)
  await build({
    entryPoints: [entry],
    bundle: true,
    treeShaking: false,
    sourcemap: true,
    minify: false,
    splitting: false,
    write: false,
    target: 'esnext',
  })
  const esbuildDuration = process.hrtime.bigint() - beforeEsbuild
  console.info(`esbuild [${name}]: `, Number(esbuildDuration / BigInt(1e6)).toFixed(2), 'ms')

  const beforeRolldown = process.hrtime.bigint()
  await rolldown(entry, {
    sourcemap: true,
  })
  const rolldownDuration = process.hrtime.bigint() - beforeRolldown
  console.info(`rolldown: [${name}]`, Number(rolldownDuration / BigInt(1e6)).toFixed(2), 'ms')

  const beforeRollup = process.hrtime.bigint()
  await rollup({
    input: entry,
    cache: false,
    treeshake: false,
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
