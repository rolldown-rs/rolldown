// foo.js
const foo = 1;
var namespaceFoo = Object.freeze({
    __proto__: null,
    foo
});

// index.js
console.log(namespaceFoo);
