// corpus: accept/a08-functions
// purpose: Calls recursive functions and a function with a parameter default.
// rule: C2
// phase: P1
// tsc: accepts
function fib(n: number): number {
  return n < 2 ? n : fib(n - 1) + fib(n - 2);
}

function scale(value: number, factor: number = 10): number {
  return value * factor;
}

export function main(): void {
  print(`${fib(20)}`);
  print(`${scale(3)} ${scale(3, 2)}`);
}
