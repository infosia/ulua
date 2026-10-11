const values = [0, -0, 1, -1, 1.5, 0.1, 0.1 + 0.2, 1 / 3, 100, 1e15, 1e16, 1e20, 1e21, 1.5e21, 1e-6, 1e-7, 1.5e-7, 123456789012345680000, 2 ** 53, 2 ** 53 + 2, 5e-324, 1.7976931348623157e308, NaN, Infinity, -Infinity];
for (const v of values) console.log(String(v));
