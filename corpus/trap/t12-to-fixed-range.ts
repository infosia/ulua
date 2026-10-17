// corpus: trap/t12-to-fixed-range
// purpose: Traps on toFixed with a digit count above 100; node throws RangeError.
// rule: C4
// phase: P7.3
// tsc: accepts
// js-comparable: no C4
// trap: invalid-argument
export function main(): void {
  print(`${(1.5).toFixed(100).length}`);
  print(`${(1.5).toFixed(101)}`);
}
