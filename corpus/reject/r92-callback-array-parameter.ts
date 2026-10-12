// corpus: reject/r92-callback-array-parameter
// purpose: Rejects a callback that takes the array as its third parameter.
// rule: C6
// phase: P2.3
// tsc: accepts
// expected-error: C6: a callback takes the value and the index only
export function main(): void {
  const xs: number[] = [1, 2];
  xs.forEach((v: number, i: number, all: number[]): void => {
    print(`${v + i + all.length}`);
  });
}
