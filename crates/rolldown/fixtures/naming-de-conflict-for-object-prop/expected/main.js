// foo.js
const foo$1 = 'foo.js';
const bar = 'foo.js';

// index.js
const foo = 'index.js';
const obj = {
    RenamedFooShouldNotChangedInObjectProp: foo$1,
    bar
};
export { foo, obj };
