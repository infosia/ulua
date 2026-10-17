// corpus: trap/t16-json-nan
// purpose: Traps on JSON.stringify of NaN; node writes null, which parses back as a different value.
// rule: C20
// phase: P7.6
// tsc: accepts
// js-comparable: no C20
// trap: json-number
type Sample = { value: number };

export function main(): void {
  const ok: Sample = { value: 1 };
  print(JSON.stringify(ok));
  const bad: Sample = { value: NaN };
  print(JSON.stringify(bad));
}
