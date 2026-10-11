// corpus: reject/r71-boolean-let-equality
// purpose: Rejects boolean equality operands; tsc can narrow a boolean let on assignment.
// rule: C7
// phase: P1
// tsc: errors TS2367
// expected-error: C7: equality does not take boolean operands
export function main(): void {
  let x: boolean = true;
  print(`${x === false}`);
}
