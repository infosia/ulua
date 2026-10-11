// corpus: reject/r19-structural-primitive
// purpose: Rejects a string, an array, and a function in a position of an object type.
// rule: C1
// phase: P2.4
// tsc: accepts
// expected-error: C1: a value of an object type must be an object
type Sized = { length: number };

function len(v: Sized): number {
  return v.length;
}

export function main(): void {
  print(`${len("héllo")} ${len([1, 2])} ${len((a: number, b: number): number => a + b)}`);
}
