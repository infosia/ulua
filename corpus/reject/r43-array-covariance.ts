// corpus: reject/r43-array-covariance
// purpose: Rejects a string array that tsc relates to an array of an object type.
// rule: C1
// phase: P2
// tsc: accepts
// expected-error: C1: array element types must be identical
type Sized = { length: number };

export function main(): void {
  const xs: string[] = ["héllo"];
  const ys: Sized[] = xs;
  print(`${ys[0].length}`);
}
