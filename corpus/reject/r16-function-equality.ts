// corpus: reject/r16-function-equality
// purpose: Rejects an identity comparison between function values.
// rule: C7
// tsc: accepts
// expected-error: C7: function values cannot be compared
function make(): () => number {
  return () => 1;
}

export function main(): void {
  print(`${make() === make()}`);
}
