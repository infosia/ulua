function make() { return () => 1; }
console.log(make() === make());
const fs = []; for (let i = 0; i < 2; i++) fs.push(() => 1);
console.log(fs[0] === fs[1]);
