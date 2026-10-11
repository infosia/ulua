// corpus: reject/r52-constructor-read-before-assign
// purpose: Rejects a use of this before every field has a value.
// rule: C13
// phase: P2.5
// tsc: accepts
// expected-error: C13: this is used before every field is assigned
class Account {
  balance: number;

  constructor(start: number) {
    this.log();
    this.balance = start;
  }

  log(): void {
    print(`${this.balance}`);
  }
}

export function main(): void {
  const a = new Account(5);
  a.log();
}
