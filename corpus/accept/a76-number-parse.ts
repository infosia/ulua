// corpus: accept/a76-number-parse
// purpose: parseInt and parseFloat prefixes, signs, whitespace, radix prefixes, and failures.
// rule: C4
// phase: P7.3
// tsc: accepts
// js-comparable: yes
function z(x: number): string {
  return `${x} ${1 / x}`;
}

export function main(): void {
  print(`${parseInt("  -0x1F", 16)} ${parseInt("0x1F", 10)} ${parseInt("123abc", 10)} ${parseInt("", 10)} ${parseInt("z", 36)} ${parseInt("1e3", 10)}`);
  print(`${parseInt("9007199254740993", 10)} ${z(parseInt("-0", 10))} ${parseInt(" \n 42", 10)} ${parseInt("101", 2)} ${parseInt("777", 8)} ${Number.parseInt("+12", 10)}`);
  print(`${parseFloat("3.14abc")} ${parseFloat(".5")} ${parseFloat("-.5e-3x")} ${parseFloat("Infinityx")} ${parseFloat("-Infinity")} ${parseFloat("1e")}`);
  print(`${parseFloat("e5")} ${parseFloat("  \n 2")} ${parseFloat("0x10")} ${parseFloat("1e400")} ${z(parseFloat("-0"))} ${Number.parseFloat("5.")}`);
}
