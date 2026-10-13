// corpus: reject/r127-host-signature
// purpose: Rejects a host function declaration whose signature differs from its grant.
// rule: C18
// phase: P3
// grants: hostAdd
// tsc: accepts
// expected-error: C18: the declaration and the grant have different signatures
declare function hostAdd(a: string, b: number): number;
export function main(): void {
  print(`${hostAdd("1", 2)}`);
}
