const [a, b] = array;
const [a1, , b2] = array;
const [a3 = aDefault, b3] = array;
const [a4, b4, ...rest4] = array;
const [a5, , b5, ...rest5] = array;
const [a6, b6, ...{ pop , push  }] = array;
const [a7, b7, ...[c7, d7]] = array;
const { _a , _b  } = obj;
const { a: _a1 , b: _b1  } = obj;
const { a: _a3 = aDefault , _b3 =bDefault  } = obj;
const { _a4 , _b4 , ..._rest4 } = obj;
const { a: _a5 , b: _b5 , ..._rest5 } = obj;
var foo_namespace = Object.freeze({
    __proto__: null,
    _a,
    _a1,
    _a3,
    _a4,
    _a5,
    _b,
    _b1,
    _b3,
    _b4,
    _b5,
    _rest4,
    _rest5,
    a,
    a1,
    a3,
    a4,
    a5,
    a6,
    a7,
    b,
    b2,
    b3,
    b4,
    b5,
    b6,
    b7,
    c7,
    d7,
    pop,
    push,
    rest4,
    rest5
});

export { foo_namespace as fooNamespace };
