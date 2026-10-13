// corpus: reject/r132-host-export
// purpose: Rejects an exported host function declaration.
// rule: C18
// phase: P3
// grants: hostAdd
// tsc: accepts
// expected-error: C18: a host function is not exported
export declare function hostAdd(a: number, b: number): number;
export function main(): void {
  print(`${hostAdd(1, 2)}`);
}
