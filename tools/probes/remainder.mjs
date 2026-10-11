const show = (v) => (Object.is(v, -0) ? "-0" : String(v));
const cases = [[-7, 3], [7, -3], [-4, 2], [5.5, 2], [5, 0], [Infinity, 2], [2, Infinity], [-2, Infinity]];
for (const [a, b] of cases) console.log(`${show(a)} % ${show(b)} = ${show(a % b)}`);
