const foo = 1;
var namespaceFoo = Object.freeze({
    __proto__: null,
    foo
});

console.log(namespaceFoo);
