// corpus: reject/r132-host-export
// purpose: Rejects an exported declaration of a host function.
// rule: C18
// phase: P3
// grants: hostAdd
// tsc: accepts
// expected-error: C18: a script declares no host function
export declare function hostAdd(a: number, b: number): number;
export function main(): void {
  print(`${hostAdd(1, 2)}`);
}
