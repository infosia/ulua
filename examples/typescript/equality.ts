// Comparing values. In TypeScript, style guides tell you to write `===`,
// because `==` converts values (`"1" == 1` is true). In Ulua both
// operands of a comparison must have the same type, so nothing is
// converted and `==` means the same as `===`. These examples use `==`.

type Point = { x: number; y: number };

function isReady(): boolean {
  return true;
}

export function main(): void {
  // Numbers, strings, and booleans compare by value.
  const sum: number = 1 + 1;
  const word: string = "ulua";
  print(`${sum == 2} ${word == "ulua"} ${isReady() != false}`);

  // A record, an array, or a class value compares by identity: two
  // values with the same fields are still two values. This is the same
  // in JavaScript.
  const a: Point = { x: 1, y: 2 };
  const b: Point = { x: 1, y: 2 };
  const c = a;
  print(`${a == b} ${a == c}`);

  // To compare the contents, compare the fields yourself.
  print(`${a.x == b.x && a.y == b.y}`);

  // A value of type `T | null` compares with `null`.
  const nickname: string | null = null;
  print(`${nickname == null}`);

  // NaN is not equal to anything, not even to itself.
  const n = 0 / 0;
  print(`${n == n}`);

  // `"1" == 1` does not compile in Ulua, because a string and a number
  // are two types. Convert one side first.
  print(`${parseInt("1", 10) == 1}`);
}
