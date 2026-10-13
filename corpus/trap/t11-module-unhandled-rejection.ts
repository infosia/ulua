// corpus: trap/t11-module-unhandled-rejection
// purpose: An unhandled rejection that the module-level statements leave stops ulua_instance_new.
// rule: C19
// phase: P4.2
// tsc: accepts
// js-comparable: no C19
// trap: uncaught-exception
async function bad(): Promise<number> {
  throw new Error("module");
}
const p = bad();
export function main(): void {
  print("main");
}
