// corpus: accept/a26-luau-keyword-names
// purpose: Uses Luau keywords and Luau global names as script binding and function names.
// rule: C14
// phase: P1
// tsc: accepts
// js-comparable: yes
function end(local: number, nil: number): number {
  return local + nil;
}

function type(then: number): number {
  return then * 2;
}

export function main(): void {
  const until = 5;
  const repeat = 6;
  const elseif = 7;
  const not = 8;
  const and = 9;
  let or = 10;
  or += 0;
  const self = 1;
  const error = 2;
  const pcall = 3;
  print(`${end(1, 2) + type(3) + until + repeat + elseif + not + and + or + self + error + pcall}`);
}
