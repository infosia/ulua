// Strings. A Ulua string holds UTF-8 bytes, so `length` and every
// position count bytes. JavaScript counts UTF-16 code units. For plain
// ASCII text the two agree; for other text they differ.

export function main(): void {
  // ASCII text: the results are the same as in JavaScript.
  const word = "hello";
  print(`${word.length} ${word.indexOf("l")} ${word.slice(1, 3)} ${word.toUpperCase()}`);

  // "é" is two bytes in UTF-8, so the text below is 6 bytes long and the
  // first "l" is at byte 3. node prints `5 2 hél` here.
  const name = "héllo";
  print(`${name.length} ${name.indexOf("l")} ${name.slice(0, 3)}`);

  // A position must fall on the start of a character. `name.slice(0, 2)`
  // would cut "é" in half, and Ulua stops the program with
  // `string-boundary` for such a position.

  // `for...of` walks the text one character at a time, as in JavaScript.
  // Count the characters this way when you need the number of
  // characters, not bytes.
  let characters = 0;
  for (const c of "日本語") {
    characters++;
  }
  // Each of these characters is 3 bytes, so Ulua prints `3 9`. node
  // prints `3 3`.
  print(`${characters} ${"日本語".length}`);

  // `Utf8.byteLength` is a Ulua addition. It gives the byte count, the
  // same number as `length`, and its name says what it counts.
  print(`${Utf8.byteLength("日本")}`);

  // Character codes agree with JavaScript. The emoji below is 4 bytes;
  // node gives 2 for its length, because it counts UTF-16 units.
  print(`${"é".codePointAt(0)} ${String.fromCodePoint(0x1f600).length}`);

  // Splitting, trimming, and searching work as in JavaScript.
  print(`${" a,b,c ".trim().split(",").join("|")} ${"ulua".startsWith("ul")} ${"ulua".includes("ua")}`);

  // Ulua does not accept `name[0]` or `a < b` between strings, because
  // a JavaScript string element is a UTF-16 unit. Use `slice` and
  // `codePointAt` in their place.
  print(`${name.slice(0, 1)} ${"a".codePointAt(0) < "b".codePointAt(0)}`);
}
