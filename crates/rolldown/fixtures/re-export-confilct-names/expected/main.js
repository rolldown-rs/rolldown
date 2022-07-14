// foo.js
const foo = 'foo.js';
var Foo = Object.freeze({
    __proto__: null,
    foo
});

// a.js

// index.js
export { Foo, Foo as RenamedFoo };
