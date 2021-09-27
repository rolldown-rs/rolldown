import { join } from 'path'

import test from 'ava'

import { rolldown } from '../index'

test('should be able to bootstrap', async (t) => {
  const output = await rolldown(join(__dirname, 'fixtures', 'main.js'))
  t.snapshot(output.toString('utf-8'))
})
