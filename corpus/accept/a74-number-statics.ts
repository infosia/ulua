// corpus: accept/a74-number-statics
// purpose: Number constants and the four Number predicates at their edges.
// rule: C4
// phase: P7.3
// tsc: accepts
// js-comparable: yes
export function main(): void {
  // MAX_SAFE_INTEGER describes the integer precision of a double.
  print(`${Number.MAX_SAFE_INTEGER} ${Number.MIN_SAFE_INTEGER} ${Number.EPSILON} ${Number.MAX_VALUE} ${Number.MIN_VALUE}`);
  print(`${Number.POSITIVE_INFINITY} ${Number.NEGATIVE_INFINITY} ${Number.NaN}`);
  print(`${Number.isNaN(NaN)} ${Number.isNaN(1)} ${Number.isFinite(Infinity)} ${Number.isFinite(-0)} ${Number.isFinite(NaN)}`);
  print(`${Number.isInteger(5)} ${Number.isInteger(5.5)} ${Number.isInteger(-0)} ${Number.isInteger(Infinity)} ${Number.isInteger(1e300)}`);
  print(`${Number.isSafeInteger(9007199254740991)} ${Number.isSafeInteger(9007199254740992)} ${Number.isSafeInteger(-9007199254740991)} ${Number.isSafeInteger(3.5)}`);
}
