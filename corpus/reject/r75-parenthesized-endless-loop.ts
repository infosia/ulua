// corpus: reject/r75-parenthesized-endless-loop
// purpose: Rejects a function with a result that ends in a loop with a parenthesized true test.
// rule: C11
// phase: P1
// tsc: errors TS2366
// expected-error: C11: a function with a result must end in a statement that cannot complete
function first(): number {
  while ((true)) {
    return 1;
  }
}

export function main(): void {
  print(`${first()}`);
}
