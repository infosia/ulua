// corpus: accept/a38-string-bytes
// purpose: Pins the byte measures of string members on non-ASCII text, a divergence from UTF-16 units.
// rule: C5
// phase: P2.4
// tsc: accepts
// js-comparable: no C5
export function main(): void {
  const s = "héllo, 世界";
  print(`${s.length} ${s.indexOf("l")} ${s.indexOf("世")} ${s.slice(0, 3)} ${s.charAt(1)} ${s.codePointAt(1)} ${s.charCodeAt(1)}`);
  print("日本語".split("").join("|"));
  let out = "";
  for (const c of "a😀é") {
    out = out + "[" + c + "]";
  }
  print(out);
  print(`${"😀".length} ${"x😀".at(1)}`);
}
