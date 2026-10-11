// corpus: reject/r02-undefined
// purpose: Rejects undefined in a type and in an expression.
// rule: C2
// tsc: accepts
// expected-error: C2: undefined is not part of the language
export function main(): void {
  const value: number | undefined = undefined;
  print(`${value === undefined}`);
}
