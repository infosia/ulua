// corpus: reject/r04-optional-property
// purpose: Rejects an optional property.
// rule: C2
// tsc: accepts
// expected-error: C2: an optional property is not part of the language
type Item = { name: string; note?: string };

export function main(): void {
  const item: Item = { name: "a" };
  print(item.note ?? "none");
}
