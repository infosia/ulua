// corpus: accept/a54-try-loop-jumps
// purpose: Leaves a try statement through break and continue to an outer loop when the block also returns.
// rule: C17
// phase: P2.7
// tsc: accepts
// js-comparable: yes
function first(): string {
  for (let i = 0; i < 2; i++) {
    try {
      if (i === 0) {
        break;
      }
      return "a";
    } catch {
      return "c";
    }
  }
  return "end";
}

function walk(): void {
  for (let i = 0; i < 3; i++) {
    try {
      if (i === 0) {
        continue;
      }
      if (i === 1) {
        break;
      }
      return;
    } finally {
      print(`fin${i}`);
    }
  }
  print("after loop");
}

function retry(): number {
  let n = 0;
  while (true) {
    try {
      n = n + 1;
      if (n < 3) {
        continue;
      }
      return n;
    } finally {
      print(`f${n}`);
    }
  }
}

export function main(): void {
  walk();
  print(first());
  print(`${retry()}`);
}
