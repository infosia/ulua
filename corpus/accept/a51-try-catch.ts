// corpus: accept/a51-try-catch
// purpose: Throws Error values through calls, methods, and callbacks, and catches them by class.
// rule: C17
// phase: P2.7
// tsc: accepts
// js-comparable: yes
class Account {
  balance: number = 0;

  withdraw(n: number): void {
    if (n > this.balance) {
      throw new RangeError("insufficient");
    }
    this.balance -= n;
  }
}

function parse(s: string): number {
  if (s.length === 0) {
    throw new TypeError("empty");
  }
  return s.length;
}

function describe(f: () => number): string {
  try {
    return `ok ${f()}`;
  } catch (e) {
    if (e instanceof RangeError) {
      return `range ${e.message}`;
    }
    if (e instanceof Error) {
      return `${e.name} ${e.message}`;
    }
    throw e;
  }
}

export function main(): void {
  const a = new Account();
  print(describe((): number => parse("abc")));
  print(describe((): number => parse("")));
  print(describe((): number => {
    a.withdraw(5);
    return 0;
  }));
  const xs: number[] = [3, 1, 2];
  print(describe((): number => {
    xs.sort((p: number, q: number): number => {
      throw new Error(`comparator ${p + q > 0}`);
    });
    return 0;
  }));
  let count = 0;
  try {
    [1, 2, 3].forEach((v: number): void => {
      count += v;
      if (v === 2) {
        throw new Error("stop");
      }
    });
  } catch {
    print(`stopped at ${count}`);
  }
  try {
    try {
      throw new SyntaxError("inner");
    } catch (e) {
      if (e instanceof SyntaxError) {
        e.name = "Renamed";
      }
      throw e;
    }
  } catch (e) {
    if (e instanceof SyntaxError) {
      print(`${e.name} ${e.message} ${e instanceof Error}`);
    }
  }
}
