var foo = 'foo.js';
var namespaceFoo = Object.freeze({
    __proto__: null,
    "default": foo
});

console.log(namespaceFoo);
