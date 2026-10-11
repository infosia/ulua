// corpus: accept/a06-if-else
// purpose: Selects branches with if, else if, else, and the conditional operator.
// rule: C3
// tsc: accepts
function sign(n: number): string {
  if (n > 0) {
    return "positive";
  } else if (n < 0) {
    return "negative";
  } else {
    return "zero";
  }
}

export function main(): void {
  print(sign(3));
  print(sign(-3));
  print(sign(0));
  const n = 5;
  print(n % 2 === 0 ? "even" : "odd");
}
