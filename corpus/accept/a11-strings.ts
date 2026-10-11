// corpus: accept/a11-strings
// purpose: Concatenates, compares, searches, and splits strings without encoding-dependent forms.
// rule: C5
// phase: P2
// tsc: accepts
// js-comparable: yes
export function main(): void {
  const name = "héllo";
  const greeting = name + ", " + "世界";
  print(greeting);
  print(`${greeting === "héllo, 世界"} ${greeting !== name}`);
  print(`${greeting.includes("llo")} ${greeting.startsWith("hé")} ${greeting.endsWith("界")}`);
  const parts = "a,b,,c".split(",");
  for (const part of parts) {
    print(`[${part}]`);
  }
}
