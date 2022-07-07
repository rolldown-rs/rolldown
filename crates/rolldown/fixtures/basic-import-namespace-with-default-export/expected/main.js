var foo = 'foo.js';
var namespaceFoo = Object.freeze({
    __proto__: null,
    "default": foo
});

// index.js
console.log(namespaceFoo);
