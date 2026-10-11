// corpus: reject/r58-bigint
// purpose: Rejects BigInt literals.
// rule: C4
// tsc: accepts
// expected-error: C4: bigint is not part of the language
export function main(): void {
  let big = 9007199254740993n;
  let big2 = 9007199254740992n;
  print(`${big === big2}`);
}
