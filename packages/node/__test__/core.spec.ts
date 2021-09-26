import test from 'ava'

import { rolldown } from '../index'

test('should be able to bootstrap', async (t) => {
  await t.notThrowsAsync(() => rolldown())
})
