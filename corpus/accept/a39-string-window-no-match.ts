// corpus: accept/a39-string-window-no-match
// purpose: A comparison window that ends inside a code point is no match, not a trap.
// rule: C5
// phase: P2.4
// tsc: accepts
// js-comparable: yes
export function main(): void {
  print(`${"café".endsWith("e")} ${"naïve".startsWith("nai")} ${"日本語".startsWith("ab")} ${"éa".endsWith("xa")} ${"café".includes("e")}`);
}
