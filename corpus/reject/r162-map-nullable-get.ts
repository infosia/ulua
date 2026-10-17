// corpus: reject/r162-map-nullable-get
// purpose: Rejects get on a Map whose value type includes null; a miss and a stored null are one nil.
// rule: C2
// phase: P7.4
// tsc: accepts
// expected-error: C2: get needs a value type without null; use has and getOr
export function main(): void {
  const m = new Map<string, number | null>();
  m.set("a", null);
  print(`${m.get("a") == null}`);
}
