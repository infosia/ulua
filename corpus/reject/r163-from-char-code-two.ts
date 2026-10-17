// corpus: reject/r163-from-char-code-two
// purpose: Rejects String.fromCharCode with two arguments; the accepted arity is one.
// rule: C5
// phase: P7.5
// tsc: accepts
// expected-error: C5: String.fromCharCode takes exactly one argument
export function main(): void {
  print(String.fromCharCode(72, 105));
}
