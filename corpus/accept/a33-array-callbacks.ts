// corpus: accept/a33-array-callbacks
// purpose: Uses the array methods with callbacks, a required reduce initial value, and a stable sort.
// rule: C6
// phase: P2.3
// tsc: accepts
// js-comparable: yes
export function main(): void {
  const xs: number[] = [5, 3, 8, 1];
  xs.forEach((v: number, i: number): void => {
    print(`${i}:${v}`);
  });
  print(xs.map((v: number): string => `<${v}>`).join(""));
  print(xs.filter((v: number): boolean => v > 2).join());
  print(`${xs.some((v: number): boolean => v > 7)} ${xs.every((v: number): boolean => v > 0)} ${xs.every((v: number): boolean => v > 1)}`);
  print(`${xs.findIndex((v: number): boolean => v > 4)} ${xs.findLastIndex((v: number): boolean => v > 4)} ${xs.findIndex((v: number): boolean => v > 9)}`);
  print(xs.flatMap((v: number, i: number): number[] => [v, i]).join());
  print(`${xs.reduce((a: number, v: number): number => a * 10 + v, 0)} ${xs.reduceRight((a: number, v: number): number => a * 10 + v, 0)}`);
  const pairs: number[][] = [[1, 0], [0, 1], [1, 2], [0, 3], [1, 4]];
  pairs.sort((a: number[], b: number[]): number => a[0] - b[0]);
  print(pairs.map((p: number[]): string => `${p[0]}${p[1]}`).join(" "));
  const nums: number[] = [10, 1, 2];
  nums.sort((a: number, b: number): number => a - b);
  print(nums.join());
}
