// corpus: accept/a20-literals
// purpose: Emits number and string literals from their values, not their spelling.
// rule: C14
// phase: P2
// tsc: accepts
// js-comparable: yes
export function main(): void {
  let big = 0xffffffffffffffffff;
  print(`${big === 4722366482869645213696}`);
  print(`${0o17} ${0b101} ${1_000} ${.5} ${5.} ${0x10} ${1e400}`);
  print("a\
b");
  print("\x41B\u{43}");
  let nul = "\0";
  let tab = "\v";
  print(`${nul === "\u0000"} ${tab === "\u000b"}`);
  print(`${Utf8.byteLength("a\r\nb")} ${Utf8.byteLength("a\0b")}`);
  print(`line one
line two`);
}
