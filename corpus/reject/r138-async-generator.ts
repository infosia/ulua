// corpus: reject/r138-async-generator
// purpose: Rejects an async generator.
// rule: C19
// phase: P4.1
// tsc: accepts
// expected-error: C19: generators are rejected
async function* count(): AsyncGenerator<number> {
  yield 1;
}
export function main(): void {
  print("x");
}
