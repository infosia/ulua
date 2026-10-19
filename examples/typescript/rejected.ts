// A program that TypeScript accepts and Ulua does not. Ulua checks each
// program before it runs and stops at the first form that it cannot
// run in the same way as JavaScript.
//
// Here the form is `**`. The runner prints the error code, the line,
// and the column, and the program does not start:
//
//   error C4 at 13:15: Numeric operator has no lowering
//
// The code (C4) names the rule in `specs/blocks/collisions.md`, where
// each rule is explained. Write `Math.pow(2, 10)` to fix this program.
export function main(): void {
  const kib = 2 ** 10;
  print(`${kib}`);
}
