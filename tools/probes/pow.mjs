const cases = [[1, NaN], [NaN, 0], [-8, 1 / 3], [2, 0.5], [-1, Infinity], [1, Infinity], [0, -1], [-0, -1]];
for (const [a, b] of cases) console.log(`${a} ** ${b} = ${a ** b}`);
