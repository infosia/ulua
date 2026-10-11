// corpus: accept/a21-member-names
// purpose: Uses member names that Luau gives a meaning as plain keys.
// rule: C14
// phase: P2.5
// tsc: accepts
// js-comparable: yes
type R = { __index: number; nil: number; end: number };

class Q {
  new: number | null = null;
  end: number = 1;

  constructor() {}

  get(): number {
    return this.new ?? 5;
  }

  __eq(other: Q): boolean {
    return other.end === this.end;
  }
}

export function main(): void {
  const a = new Q();
  const b = new Q();
  print(`${a === b} ${a.get()} ${a.end} ${a.__eq(b)}`);
  a.new = 3;
  print(`${a.get()}`);
  const r: R = { __index: 1, nil: 2, end: 3 };
  print(`${r.__index + r.nil + r.end}`);
}
