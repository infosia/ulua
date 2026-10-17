// corpus: reject/r165-json-map
// purpose: Rejects JSON.stringify of a Map; node writes {} and loses every entry.
// rule: C20
// phase: P7.6
// tsc: accepts
// expected-error: C20: JSON.stringify of a Map is rejected
export function main(): void {
  const m = new Map<string, number>();
  m.set("a", 1);
  print(JSON.stringify(m));
}
