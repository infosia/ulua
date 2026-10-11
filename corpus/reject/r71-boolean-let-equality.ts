// corpus: reject/r71-boolean-let-equality
// purpose: Rejects a boolean let binding as an equality operand; tsc narrows it on assignment.
// rule: C7
// phase: P1
// tsc: errors TS2367
// expected-error: C7: a boolean let binding is not an equality operand
export function main(): void {
  let x: boolean = true;
  print(`${x === false}`);
}
