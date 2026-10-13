// corpus: accept/a55-host-functions
// purpose: Calls each host fixture with each value type.
// rule: C18
// phase: P3
// grants: hostAdd, hostConcat, hostNot, hostLog
// tsc: accepts
// js-comparable: yes
declare function hostAdd(a: number, b: number): number;
declare function hostConcat(left: string, right: string): string;
declare function hostNot(b: boolean): boolean;
declare function hostLog(text: string): void;
export function main(): void {
  print(`${hostAdd(1.5, 2)}`);
  print(`${hostAdd(0.1, 0.2)}`);
  print(hostConcat("é", "x"));
  print(`${hostConcat("", "").length}`);
  const t: boolean = hostNot(false);
  print(`${t}`);
  if (hostNot(true)) {
    print("unreachable");
  } else {
    print("else");
  }
  hostLog("one");
  hostLog(hostConcat("a", "b"));
}
