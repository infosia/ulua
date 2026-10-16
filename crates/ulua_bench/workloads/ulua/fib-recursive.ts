function fib(n: number): number {
  if (n < 2) { return n; }
  return fib(n - 1) + fib(n - 2);
}
export function main(): number { return fib(31) % 4294967296; }
