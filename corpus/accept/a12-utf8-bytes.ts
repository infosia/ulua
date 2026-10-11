// corpus: accept/a12-utf8-bytes
// purpose: Measures UTF-8 byte length through the prelude name.
// rule: C5
// phase: P2
// tsc: accepts
// js-comparable: yes
export function main(): void {
  print(`${Utf8.byteLength("abc")} ${Utf8.byteLength("héllo")} ${Utf8.byteLength("日本")} ${Utf8.byteLength("😀")}`);
}
