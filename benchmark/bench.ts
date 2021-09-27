import { join } from 'path'

import b from 'benny'
import { rollup } from 'rollup'

import { rolldown } from '../packages/node'

const ENTRY = join(__dirname, 'fixtures', 'main.js')

async function run() {
  await b.suite(
    'Bundle simple file',

    b.add('Rollup', async () => {
      await rollup({
        input: ENTRY,
        cache: false,
        treeshake: false,
      })
    }),

    b.add('Rolldown', async () => {
      await rolldown(ENTRY)
    }),

    b.cycle(),
    b.complete(),
  )
}

run().catch((e) => {
  console.error(e)
})
