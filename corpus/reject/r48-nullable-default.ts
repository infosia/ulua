// corpus: reject/r48-nullable-default
// purpose: Rejects a default on a parameter whose type includes null.
// rule: C2
// tsc: accepts
// expected-error: C2: a parameter with a default cannot include null
function show(x: number | null = 5): string {
  return `${x ?? -1}`;
}

export function main(): void {
  print(show(null));
}
