// corpus: accept/a81-string-from-code
// purpose: String.fromCharCode and String.fromCodePoint from ASCII to the astral planes.
// rule: C5
// phase: P7.5
// tsc: accepts
// js-comparable: yes
export function main(): void {
  print(`${String.fromCharCode(72)}${String.fromCharCode(105)} ${String.fromCharCode(233)} ${String.fromCharCode(0x3042)} ${String.fromCharCode(65601)}`);
  print(`${String.fromCodePoint(65)} ${String.fromCodePoint(0x1F600)} ${String.fromCodePoint(0x10FFFF) === String.fromCodePoint(1114111)} ${String.fromCodePoint(0x7F) === String.fromCharCode(127)}`);
  let s = "";
  for (let cp = 0x41; cp < 0x47; cp++) {
    s = s + String.fromCodePoint(cp);
  }
  print(s.toLowerCase());
}
