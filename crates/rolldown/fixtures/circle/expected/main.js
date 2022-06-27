console.log('before import index');
import { index as index1 } from './index';
console.log('index is ' + index1);
export let foo = index1 + 1;
console.log('before import foo');
import { foo as foo1 } from './foo';
console.log('foo is ' + foo1);
export let index = foo1 + 1;
