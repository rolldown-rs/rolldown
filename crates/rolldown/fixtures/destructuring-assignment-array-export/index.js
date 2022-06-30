export const [a, b] = array;
export const [a1, , b2] = array;
export const [a3 = aDefault, b3] = array;
export const [a4, b4, ...rest4] = array;
export const [a5, , b5, ...rest5] = array;
export const [a6, b6, ...{ pop, push }] = array;
export const [a7, b7, ...[c7, d7]] = array;
