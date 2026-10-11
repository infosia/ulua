// corpus: reject/r45-index-signature
// purpose: Rejects an index signature type.
// rule: C6
// tsc: accepts
// expected-error: C6: an index signature type is not part of the language
type Table = { [key: string]: number };

export function main(): void {
  const d: Table = { a: 1 };
  print(`${d["b"] + 1}`);
}
