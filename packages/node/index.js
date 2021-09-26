const { join } = require('path')

const { loadBinding } = require('@node-rs/helper')

module.exports = loadBinding(join(__dirname, '..', '..'), 'rolldown', '@rolldown/core')
