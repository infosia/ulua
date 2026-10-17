// corpus: accept/a80-string-case
// purpose: toUpperCase and toLowerCase over ASCII, Latin, Greek final sigma, sharp s, dotted I, and ligatures.
// rule: C5
// phase: P7.5
// tsc: accepts
// js-comparable: yes
export function main(): void {
  print(`${"Hello, World!".toUpperCase()} ${"Hello, World!".toLowerCase()}`);
  print(`${"café".toUpperCase()} ${"ÉCOLE".toLowerCase()} ${"straße".toUpperCase()}`);
  print(`${"ΟΔΥΣΣΕΥΣ".toLowerCase()} ${"ΑΣ Α".toLowerCase()} ${"Σ".toLowerCase()}`);
  print(`${"İstanbul".toLowerCase()} ${"ﬁne".toUpperCase()} ${"ǅ".toLowerCase()} ${"ǆ".toUpperCase()}`);
  print(`${"".toUpperCase() === ""} ${"123 !?".toLowerCase()} ${"日本".toUpperCase()}`);
}
