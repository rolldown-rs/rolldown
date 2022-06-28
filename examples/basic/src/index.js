import './side-effect'
import './pure'
import  { foo, foo as foo2, bar } from './foo'
// import  * as namespaceFoo from './foo'
console.log(foo, foo2, bar, namespaceFoo)