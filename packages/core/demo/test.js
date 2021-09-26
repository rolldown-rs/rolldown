const rollup = require('../dist/rollup')

rollup(__dirname + '/main.js').then((res) => {
  res.write('bundle.js')
})
