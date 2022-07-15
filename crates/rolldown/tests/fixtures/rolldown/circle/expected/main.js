// foo.js
console.log('before import index');
console.log('index is ' + index);
let foo = index + 1;

// index.js
console.log('before import foo');
console.log('foo is ' + foo);
let index = foo + 1;
export { index };
