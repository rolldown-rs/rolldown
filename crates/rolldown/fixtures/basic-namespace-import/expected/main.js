const foo = 1;

var namespaceFoo = Object.freeze({
	__proto__: null,
	foo: foo
});

console.log(namespaceFoo);