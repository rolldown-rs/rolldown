const path = require('path')

const { rolldown } = require('./node-binding')

async function main() {
  const _code = await rolldown(path.resolve(__dirname, './node/__test__/fixtures/main.js'))
}
main()
