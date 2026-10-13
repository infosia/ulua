// corpus: reject/r124-unreachable-arrow
// purpose: Rejects an arrow function at a point that no edge reaches.
// rule: C16
// phase: P2.7
// tsc: errors TS18046
// expected-error: C16: an arrow function at a point that no edge reaches
function f(): string {
  let log = "";
  throw new Error("a");
  log += ((): string => {
    try {
      log += "7";
    } catch (e) {
      if (e instanceof SyntaxError) {
        log += e.message;
      }
    }
    return "A";
  })();
  return log;
}
export function main(): void {
  try {
    print(f());
  } catch {
    print("c");
  }
}
