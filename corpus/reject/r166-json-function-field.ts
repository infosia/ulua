// corpus: reject/r166-json-function-field
// purpose: Rejects JSON.stringify of a record with a function field; node omits the field.
// rule: C20
// phase: P7.6
// tsc: accepts
// expected-error: C20: JSON.stringify of a function type is rejected
type Handler = { name: string; run: () => number };

export function main(): void {
  const h: Handler = { name: "h", run: (): number => 1 };
  print(JSON.stringify(h));
}
