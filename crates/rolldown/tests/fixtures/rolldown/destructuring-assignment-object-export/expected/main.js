// index.js
const { a , b  } = obj;
const { a: a1 , b: b1  } = obj;
const { a: a3 = aDefault , b3 =bDefault  } = obj;
const { a4 , b4 , ...rest4 } = obj;
const { a: a5 , b: b5 , ...rest5 } = obj;
export { a, a1, a3, a4, a5, b, b1, b3, b4, b5, rest4, rest5 };
