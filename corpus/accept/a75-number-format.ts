// corpus: accept/a75-number-format
// purpose: toFixed, toExponential, toPrecision, and toString(radix) at ties, zeros, large values, and fractions.
// rule: C4
// phase: P7.3
// tsc: accepts
// js-comparable: yes
export function main(): void {
  print(`${(2.5).toFixed(0)} ${(0.5).toFixed()} ${(-2.5).toFixed(0)} ${(0.25).toFixed(1)} ${(1.005).toFixed(2)} ${(-0).toFixed(2)} ${(-0.001).toFixed(2)}`);
  print(`${(1e21).toFixed(2)} ${(123.456).toFixed(10)} ${NaN.toFixed(2)} ${(-Infinity).toFixed(1)} ${(0.000001).toFixed(7)} ${(5e-324).toFixed(3)}`);
  print(`${(0).toExponential(2)} ${(123456).toExponential(2)} ${(1.5e-7).toExponential()} ${(-1.25).toExponential(1)} ${(1e21).toExponential()} ${(255).toExponential(0)}`);
  print(`${(123.456).toPrecision(4)} ${(0.000123).toPrecision(2)} ${(123456).toPrecision(2)} ${(1e21).toPrecision(3)} ${(2.5).toPrecision(1)} ${(-0).toPrecision(3)}`);
  print(`${(255).toString(16)} ${(-255).toString(2)} ${(0.5).toString(2)} ${(0.1).toString(3)} ${(1234.5678).toString(36)} ${(1e21).toString(36)}`);
  print(`${NaN.toString(2)} ${(-0).toString(2)} ${(9007199254740992).toString(7)} ${(10).toString(10)} ${(0.1).toString(10)} ${(-1.5).toString(16)}`);
}
