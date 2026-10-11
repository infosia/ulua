// corpus: accept/a19-string-edges
// purpose: Pins empty needles and separator edge cases of the accepted string operations.
// rule: C5
// phase: P2.3
// tsc: accepts
// js-comparable: yes
function show(parts: string[]): string {
  let out = "";
  for (const p of parts) {
    out = out + "[" + p + "]";
  }
  return out;
}

export function main(): void {
  const s = "abc";
  print(`${s.includes("")} ${s.startsWith("")} ${s.endsWith("")}`);
  print(`${"".includes("")} ${"".endsWith("")} ${s.endsWith("abcd")} ${s.startsWith("abcd")}`);
  print(show("a,b,".split(",")));
  print(show("aaa".split("aa")));
  print(show(",".split(",")));
  print(show("".split(",")));
  print(show("日本語".split("本")));
}
