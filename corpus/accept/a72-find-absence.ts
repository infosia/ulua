// corpus: accept/a72-find-absence
// purpose: find and findLast hits and misses, tested with == null, != null narrowing, ??, and ?. on a record.
// rule: C2, C6
// phase: P7.2
// tsc: accepts
// js-comparable: yes
type Item = { id: number; name: string };

export function main(): void {
  const xs: number[] = [4, 9, 16, 25];
  const big = xs.find((v: number): boolean => v > 10);
  const huge = xs.find((v: number): boolean => v > 100);
  const last = xs.findLast((v: number, i: number): boolean => i < 3);
  const none = xs.findLast((v: number): boolean => v < 0);
  print(`${big == null} ${huge == null} ${big != null} ${huge != null}`);
  print(`${big ?? -1} ${huge ?? -1} ${last ?? -1} ${none ?? -1}`);
  if (big != null) {
    const twice: number = big * 2;
    print(`${twice}`);
  }
  let found = xs.find((v: number): boolean => v === 9);
  found = xs.find((v: number): boolean => v === 7);
  print(`${found == null}`);
  found = 5;
  print(`${found ?? 0}`);
  const items: Item[] = [{ id: 1, name: "one" }, { id: 2, name: "two" }];
  const two = items.find((it: Item): boolean => it.id === 2);
  const three = items.find((it: Item): boolean => it.id === 3);
  print(`${two?.name ?? "none"} ${three?.name ?? "none"}`);
  const words: string[] = ["a", "bb", "ccc"];
  const w = words.findLast((s: string): boolean => s.length < 3);
  print(`${w ?? ""}`);
}
