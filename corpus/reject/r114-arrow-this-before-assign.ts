// corpus: reject/r114-arrow-this-before-assign
// purpose: Rejects an arrow that reads this before every field is assigned.
// rule: C13
// phase: P2.6
// tsc: accepts
// expected-error: C13: this is used before every field is assigned
class Late {
  v: number;
  read: () => number;

  constructor() {
    this.read = (): number => this.v;
    print(`${this.read()}`);
    this.v = 1;
  }
}

export function main(): void {
  const l = new Late();
  print(`${l.read()}`);
}
