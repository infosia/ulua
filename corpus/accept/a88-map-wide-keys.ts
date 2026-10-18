// corpus: accept/a88-map-wide-keys
// purpose: Map and Set number keys outside the int range: infinities, large magnitudes, and fractions near 2^31.
// rule: C6
// phase: P7
// tsc: accepts
// js-comparable: yes
function keys(): number[] {
  const inf = 1 / 0;
  return [
    inf,
    -inf,
    1e300,
    -1e300,
    2147483648,
    2147483647,
    -2147483648,
    -2147483649,
    2147483648.5,
    -2147483648.5,
    9007199254740992,
    0.5,
  ];
}

function map(): string {
  const m = new Map<number, number>();
  const ks = keys();
  for (let i = 0; i < ks.length; i++) {
    m.set(ks[i], i);
  }
  m.set(1 / 0, 100);
  let out = `${m.size}`;
  for (const k of m.keys()) {
    out += ` ${k}=${m.getOr(k, -1)}`;
  }
  out += ` ${m.has(-1 / 0)} ${m.has(2147483649)} ${m.getOr(1e300, -1)}`;
  out += ` ${m.delete(-2147483649)} ${m.delete(-2147483649)} ${m.size}`;
  return out;
}

function set(): string {
  const ks = keys();
  const s = new Set<number>(ks);
  s.add(1e300);
  s.add(4294967296);
  let out = `${s.size}`;
  for (const k of s) {
    out += ` ${k}`;
  }
  out += ` ${s.has(4294967296)} ${s.has(4294967297)} ${s.delete(1 / 0)} ${s.has(1 / 0)}`;
  const copy = new Set<number>(s);
  return `${out} ${copy.size} ${copy.has(-1 / 0)}`;
}

export function main(): void {
  print(map());
  print(set());
}
