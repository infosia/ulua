// Ambient declarations for Ulua programs.
// `tsc` checks every accepted program against this file and the ES2022
// library. The Ulua compiler accepts a subset of that library.

/** Writes `text` and one line feed to the host output capability. */
declare function print(text: string): void;

/**
 * Ulua traps where JavaScript gives `undefined` for these members
 * (collisions.md C6.9), so their result type is the element type.
 */
interface Array<T> {
  pop(): T;
  shift(): T;
  at(index: number): T;
  /** ES2023; the project library is ES2022. */
  findLastIndex(predicate: (value: T, index: number) => boolean): number;
}

/**
 * Ulua traps where JavaScript gives `undefined` for these members
 * (collisions.md C5), so their result types exclude `undefined`.
 */
interface String {
  at(index: number): string;
  codePointAt(pos: number): number;
}

/** Byte operations on UTF-8 strings. JavaScript has no such names. */
declare namespace Utf8 {
  /** Returns the number of UTF-8 bytes in `text`. */
  function byteLength(text: string): number;
}
