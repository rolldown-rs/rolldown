const foo = 1;
const namespaceFoo = Object.freeze({
    __proto__: null,
    foo: foo
});

console.log(namespaceFoo);
