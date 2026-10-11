// corpus: accept/a09-closures
// purpose: Captures bindings in closures, with one binding per loop iteration.
// rule: C1, C6, C9
// phase: P2.2
// tsc: accepts
// js-comparable: yes
function counter(): () => number {
  let count = 0;
  return (): number => {
    count += 1;
    return count;
  };
}

export function main(): void {
  const next = counter();
  next();
  next();
  print(`${next()}`);
  const readers: (() => number)[] = [];
  for (let i = 0; i < 3; i++) {
    readers.push(() => i * 10);
  }
  for (const read of readers) {
    print(`${read()}`);
  }
}
