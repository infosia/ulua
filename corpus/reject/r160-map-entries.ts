// corpus: reject/r160-map-entries
// purpose: Rejects Map entries, which needs a tuple type.
// rule: C6
// phase: P7.4
// tsc: accepts
// expected-error: C6: entries needs a tuple type
export function main(): void {
  const m = new Map<string, number>();
  for (const e of m.entries()) {
    print(`${e[1]}`);
  }
}
