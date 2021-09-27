import { build } from 'esbuild'
import { rollup } from 'rollup'

import { rolldown } from '../packages/node'

const ENTRY = require.resolve('lodash-es')

async function bench() {
  const beforeEsbuild = process.hrtime.bigint()
  await build({
    entryPoints: [ENTRY],
    bundle: true,
    sourcemap: false,
    minify: false,
    splitting: false,
    write: false,
  })
  const esbuildDuration = process.hrtime.bigint() - beforeEsbuild
  console.info('esbuild: ', Number(esbuildDuration / BigInt(1e6)).toFixed(2), 'ms')

  const beforeRollup = process.hrtime.bigint()
  await rollup({
    input: ENTRY,
    cache: false,
  })
  const rollupDuration = process.hrtime.bigint() - beforeRollup
  console.info('rollup: ', Number(rollupDuration / BigInt(1e6)).toFixed(2), 'ms')

  const beforeRolldown = process.hrtime.bigint()
  await rolldown(ENTRY)
  const rolldownDuration = process.hrtime.bigint() - beforeRolldown
  console.info('rolldown: ', Number(rolldownDuration / BigInt(1e6)).toFixed(2), 'ms')
}

bench().catch((e) => {
  console.error(e)
  throw e
})
