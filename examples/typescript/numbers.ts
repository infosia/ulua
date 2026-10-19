// Numbers in Ulua work as numbers in JavaScript: every `number` is a
// 64-bit floating-point value. This script works with a few prices and
// shows the few places where you write something different.

// A function that averages a list. Nothing here is new for a TypeScript
// developer.
function average(values: number[]): number {
  let sum = 0;
  for (const v of values) {
    sum += v;
  }
  return sum / values.length;
}

export function main(): void {
  const prices: number[] = [2.5, 4, 1.25];
  print(`average price ${average(prices)}`);

  // Arithmetic gives the same results as in JavaScript, including the
  // well-known rounding of 0.1 + 0.2 and the sign of `%`.
  print(`${0.1 + 0.2} ${7 % 3} ${-7 % 3}`);

  // Division by zero does not stop the program. It gives an infinity or
  // NaN, as in JavaScript.
  print(`${1 / 0} ${0 / 0}`);

  // TypeScript lets you write `2 ** 10`. Ulua does not accept the `**`
  // operator, because the Luau operator behind it gives a different
  // result from JavaScript for some inputs. `Math.pow` gives the JavaScript result.
  print(`${Math.pow(2, 10)}`);

  // `Math.max` and `Math.min` take exactly two numbers in Ulua.
  print(`${Math.max(3, 8)} ${Math.min(3, 8)}`);

  // To read a number from text, use `parseFloat`, or `parseInt` with a
  // radix. Ulua does not accept `Number("42")`, because `Number` converts
  // many kinds of values in surprising ways.
  print(`${parseInt("ff", 16)} ${parseInt("42", 10)} ${parseFloat("2.5kg")}`);

  // Text that is not a number gives NaN. A NaN is never equal to itself,
  // so `n != n` is a simple test for NaN.
  const n = parseInt("abc", 10);
  print(`${n} ${n != n}`);

  // The `Number` helpers work as in JavaScript.
  print(`${Number.isInteger(4)} ${(3.14159).toFixed(2)}`);
}
