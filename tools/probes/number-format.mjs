// Number to string over a seeded sample. Each line: category and String(v).
// The .luau pair prints the same values; compare the two outputs line by line.
let x = 2463534242;
function rnd() {
  x ^= x << 13; x >>>= 0;
  x ^= x >>> 17;
  x ^= x << 5; x >>>= 0;
  return x;
}
const view = new DataView(new ArrayBuffer(8));
const pow10 = [1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16, 1e17, 1e18, 1e19, 1e20, 1e21, 1e22];
const N = 20000;
const out = [];
for (let i = 0; i < N; i++) {
  view.setUint32(0, rnd(), true);
  view.setUint32(4, rnd(), true);
  out.push("bits " + String(view.getFloat64(0, true)));
}
for (let i = 0; i < N; i++) {
  const v = rnd() * 2097152 + (rnd() % 2097152);
  out.push("int " + String(i % 2 === 0 ? v : -v));
}
for (let i = 0; i < N; i++) {
  const v = (rnd() % 100000) / pow10[rnd() % 13];
  out.push("dec " + String(v));
}
for (let i = 0; i < N; i++) {
  const m = rnd() / 4294967296 * 9 + 1;
  const e = rnd() % 10;
  const v = e < 5 ? m / pow10[5 + e] : m * pow10[13 + e];
  out.push("edge " + String(v));
}
const specials = [0, -0, NaN, Infinity, -Infinity, 5e-324, -5e-324, 1.7976931348623157e308, 1e21, 999999999999999900000, 1e-7, 1e-6, 0.000001, 1.5e-7, 2 ** 53, 2 ** 53 + 2, -1e21, -1e-7, 2.2250738585072014e-308];
for (const v of specials) out.push("special " + String(v));
console.log(out.join("\n"));
