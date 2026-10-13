// corpus: reject/r141-async-array-callback
// purpose: Rejects an async arrow as an array method callback.
// rule: C19
// phase: P4.1
// tsc: accepts
// expected-error: C19: an async arrow is not an array method argument
export function main(): void {
  const xs: number[] = [1, 2];
  xs.forEach(async (x: number): Promise<void> => {
    print(`${x}`);
  });
}
