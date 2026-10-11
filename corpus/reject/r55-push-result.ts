// corpus: reject/r55-push-result
// purpose: Rejects the result of push used as a value.
// rule: C11
// tsc: accepts
// expected-error: C11: push is an expression statement only
export function main(): void {
  const xs: number[] = [1];
  print(`${xs.push(2)}`);
}
