// corpus: accept/a50-field-update
// purpose: Increments and decrements record fields as statements and in a for update.
// rule: C12
// phase: P2.5
// tsc: accepts
// js-comparable: yes
type R = { v: number; n: number };

export function main(): void {
  const o: R = { v: 1, n: 0 };
  o.v++;
  ++o.v;
  o.v--;
  for (let i = 0; i < 3; o.n++) {
    i += 1;
  }
  const rs: R[] = [{ v: 10, n: 0 }];
  rs[0].v++;
  print(`${o.v} ${o.n} ${rs[0].v}`);
}
