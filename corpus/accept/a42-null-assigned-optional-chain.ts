// corpus: accept/a42-null-assigned-optional-chain
// purpose: Reads optional chains on references that tsc narrows to null after an assignment of null.
// rule: C16
// phase: P2.5
// tsc: exception TS2339
// js-comparable: yes
type Box = { value: number };

export function main(): void {
  let s: string | null = "abc";
  print(`${s?.length ?? -1}`);
  s = null;
  print(`${s?.length ?? -1}`);
  let b: Box | null = null;
  print(`${b?.value ?? 0}`);
  b = { value: 4 };
  print(`${b?.value ?? 0}`);
}
