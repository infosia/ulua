// Arrays. Most array code reads the same as in TypeScript. The one big
// difference: reading past the end of an array stops the program. In
// JavaScript the read gives `undefined`, which is not a `number`, so a
// bug can travel far before you notice it. Ulua stops at the bad read.

export function main(): void {
  const scores: number[] = [30, 10, 20];
  scores.push(50);

  // Writing at index `length` adds an element, as in JavaScript.
  scores[scores.length] = 40;

  // `sort` needs a compare function in Ulua. Without one, JavaScript
  // sorts numbers as text, which is a common surprise.
  scores.sort((a: number, b: number): number => a - b);
  print(scores.join(","));

  // `filter`, `map`, and `reduce` work as in JavaScript.
  const high = scores.filter((s: number): boolean => s >= 30).map((s: number): number => s / 10);
  const total = scores.reduce((sum: number, s: number): number => sum + s, 0);
  print(`${high.join(",")} ${total}`);

  // `slice`, `indexOf`, and `includes` work as in JavaScript.
  print(`${scores.slice(1, 3).join(",")} ${scores.indexOf(40)} ${scores.includes(99)}`);

  // `pop` removes and returns the last element.
  print(`${scores.pop()} ${scores.length}`);

  // Check the length before you read an index that can be missing.
  const i = 10;
  if (i < scores.length) {
    print(`${scores[i]}`);
  } else {
    print(`no score at ${i}`);
  }

  // Without the check, the read stops the program here. node prints
  // `undefined` and goes on. `pop` on an empty array and a write past
  // the end stop the program in the same way.
  print(`${scores[i]}`);
  print("this line never runs");
}
