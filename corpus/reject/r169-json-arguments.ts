// corpus: reject/r169-json-arguments
// purpose: Rejects JSON.stringify with a space argument; the accepted arity is one.
// rule: C20
// phase: P7.6
// tsc: accepts
// expected-error: C20: JSON.stringify takes exactly one argument
export function main(): void {
  print(JSON.stringify([1, 2], null, 2));
}
