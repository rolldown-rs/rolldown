import { foo as RenamedFooShouldNotChangedInObjectProp, bar } from './foo'
export const foo = 'index.js'
export const obj = {
  RenamedFooShouldNotChangedInObjectProp,
  bar,
}
