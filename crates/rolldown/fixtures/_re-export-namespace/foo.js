export const [a, b] = array;
export const [a1, , b2] = array;
export const [a3 = aDefault, b3] = array;
export const [a4, b4, ...rest4] = array;
export const [a5, , b5, ...rest5] = array;
export const [a6, b6, ...{ pop, push }] = array;
export const [a7, b7, ...[c7, d7]] = array;

export const { _a, _b } = obj;
export const { a: _a1, b: _b1 } = obj;
export const { a: _a3 = aDefault, _b3 = bDefault } = obj;
export const { _a4, _b4, ..._rest4 } = obj;
export const { a: _a5, b: _b5, ..._rest5 } = obj;