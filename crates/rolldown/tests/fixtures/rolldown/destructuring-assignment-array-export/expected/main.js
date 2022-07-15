// index.js
const [a, b] = array;
const [a1, , b2] = array;
const [a3 = aDefault, b3] = array;
const [a4, b4, ...rest4] = array;
const [a5, , b5, ...rest5] = array;
const [a6, b6, ...{ pop , push  }] = array;
const [a7, b7, ...[c7, d7]] = array;
export { a, a1, a3, a4, a5, a6, a7, b, b2, b3, b4, b5, b6, b7, c7, d7, pop, push, rest4, rest5 };
