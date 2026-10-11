// corpus: reject/r66-rest-parameter
// purpose: Rejects a rest parameter and a spread argument.
// rule: C11
// tsc: accepts
// expected-error: C11: a parameter is an identifier with a type
function sum(...xs: number[]): number {
  let total = 0;
  for (const x of xs) {
    total += x;
  }
  return total;
}

export function main(): void {
  const ys: number[] = [1, 2];
  print(`${sum(...ys)}`);
}
