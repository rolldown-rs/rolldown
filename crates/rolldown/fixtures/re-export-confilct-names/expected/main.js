// a.js
const a = 'a.js';
var a$1 = 'default a.js';
var a$2 = Object.freeze({
    __proto__: null,
    a,
    "default": a$1
});

// b.js
var b = Object.freeze({
    __proto__: null,
    a: a$2,
    aInB: a,
    "default": a$1
});

// index.js
export { a$2 as a, a as aInB, b };
