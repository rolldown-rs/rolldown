import { join } from 'path'

import test from 'ava'

import { rolldown } from '../index'

test('should be able to bootstrap', async (t) => {
  const code = await rolldown(join(__dirname, 'fixtures', 'main.js'))
  t.snapshot(code)
})
