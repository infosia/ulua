// corpus: reject/r167-json-parse-no-target
// purpose: Rejects JSON.parse without a contextual target type.
// rule: C20
// phase: P7.6
// tsc: accepts
// expected-error: C20: JSON.parse needs a target type
export function main(): void {
  const v = JSON.parse("1");
  print(`${v}`);
}
