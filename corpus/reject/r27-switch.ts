// corpus: reject/r27-switch
// purpose: Rejects a switch statement.
// rule: C8
// tsc: accepts
// expected-error: C8: switch has no lowering
function name(n: number): string {
  switch (n) {
    case 1:
      return "one";
    default:
      return "other";
  }
}

export function main(): void {
  print(name(1));
}
