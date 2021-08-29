'use strict'

function add(a, b) {
  return a + b
}

function mul(a, b) {
  let result = 0
  for (let i = 0; i < a; i++) {
    result = add(result, b)
  }

  return result
}

;(window || global).console.log(mul(8, 9))
