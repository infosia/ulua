// corpus: accept/a29-loose-equality
// purpose: Uses == and != with the results and the operand rules of === and !==.
// rule: C7
// phase: P1
// tsc: accepts
function classify(n: number): string {
  if (n == 0) {
    return "zero";
  }
  if (n != n) {
    return "nan";
  }
  return "other";
}

function skip(limit: number): number {
  let sum = 0;
  for (let i = 0; i < limit; i++) {
    if (i == 3) {
      continue;
    }
    sum = sum + i;
  }
  return sum;
}

export function main(): void {
  const zero = 0;
  print(`${classify(0)} ${classify(-0)} ${classify(zero / zero)} ${classify(2)}`);
  print(`${skip(5)}`);
}
