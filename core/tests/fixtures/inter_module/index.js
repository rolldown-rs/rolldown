import { foo as foo2, bar } from "./foo"

const foo = () => {
   console.log("I'm the real foo function")
}

(function() {
   foo();
   console.log("foo", foo2)
   console.log("bar", bar)
})();