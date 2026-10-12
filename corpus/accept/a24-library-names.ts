// corpus: accept/a24-library-names
// purpose: Names bindings like Luau library globals that lowerings use.
// rule: C14
// phase: P2.4
// tsc: accepts
// js-comparable: yes
export function main(): void {
  const math: number[] = [7];
  const table = "t";
  const string = -7;
  const tostring = 0;
  print(`${math[0] % 3} ${string % 3} ${table + "x"} ${tostring}`);
  const fmod = (a: number, b: number): number => a + b;
  print(`${fmod(1, 2)} ${5.5 % 2}`);
}
