// corpus: reject/r13-void-value
// purpose: Rejects a binding of a void call result.
// rule: C2
// phase: P2.1
// tsc: accepts
// expected-error: C2: a void expression is a statement only
function log(text: string): void {
  print(text);
}

export function main(): void {
  const result = log("a");
  print(`${result == null}`);
}
