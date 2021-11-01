const { join } = require('path')

const { loadBinding } = require('@node-rs/helper')

const { rolldown } = loadBinding(join(__dirname, '..'), 'rolldown', '@rolldown/core')

module.exports.rolldown = function (entry, options = {}) {
  return rolldown(entry, Buffer.from(JSON.stringify(options)))
}
