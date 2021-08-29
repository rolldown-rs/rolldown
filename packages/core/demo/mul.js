import add from './add'

export default function mul(a, b) {
  let result = 0
  for (let i = 0; i < a; i++) {
    result = add(result, b)
  }

  return result
}
