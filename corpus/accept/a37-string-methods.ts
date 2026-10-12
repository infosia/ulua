// corpus: accept/a37-string-methods
// purpose: Uses string concatenation with numbers and booleans and the string members on ASCII text.
// rule: C4, C5
// phase: P2.4
// tsc: accepts
// js-comparable: yes
export function main(): void {
  const n = 1e21;
  print("x=" + 3 + " " + 0.1 + " " + n + " " + true + " " + -0);
  let s = "a";
  s += 2;
  s += false;
  print(s);
  const t = "hello, world";
  print(`${t.length} ${t.slice(-5)} ${t.slice(2, -3)} ${t.substring(5, 2)} ${t.substr(-5, 3)}`);
  print(`${t.indexOf("o")} ${t.indexOf("o", 5)} ${t.lastIndexOf("o")} ${t.lastIndexOf("o", 5)} ${t.includes("lo", 3)} ${t.startsWith("world", 7)} ${t.endsWith("hello", 5)}`);
  print(`${t.charAt(1)} [${t.charAt(99)}] ${t.charCodeAt(0)} ${t.codePointAt(4)} ${t.at(-1)}`);
  print(t.split(", ").join("|"));
  print("a,b,,c".split(",", 2).join("|"));
  print("abc".split("").join("|"));
  print(`[${"  \t pad \n".trim()}] [${"  x ".trimStart()}] [${"  x ".trimEnd()}]`);
  print(`${"ab".repeat(3)} [${"7".padStart(3, "0")}] [${"7".padEnd(3)}] [${"abc".padStart(5, "")}]`);
  print(`${"a-b-c".replace("-", "+")} ${"a-b-c".replaceAll("-", "+")} ${"x".replace("x", "[$&$$]")} ${"ab".replaceAll("", ".")}`);
  print("con".concat("cat"));
  let out = "";
  for (const c of "abc") {
    out = out + c + ";";
  }
  print(out);
}
