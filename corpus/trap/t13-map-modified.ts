// corpus: trap/t13-map-modified
// purpose: Traps when a forEach callback adds an entry to the map it traverses; node visits the new entry.
// rule: C6
// phase: P7.4
// tsc: accepts
// js-comparable: no C6
// trap: map-modified
export function main(): void {
  const m = new Map<string, number>();
  m.set("x", 1);
  m.forEach((v: number, k: string): void => {
    print(k);
    m.set("y", 2);
  });
  print("after");
}
