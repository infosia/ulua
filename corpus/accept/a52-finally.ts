// corpus: accept/a52-finally
// purpose: Runs finally on each exit and lets a completion in finally replace the earlier one.
// rule: C17
// phase: P2.7
// tsc: accepts
// js-comparable: yes
function keep(): string {
  try {
    return "try";
  } finally {
    print("finally after return");
  }
}

function replaceReturn(): string {
  try {
    return "try";
  } finally {
    return "finally";
  }
}

function replaceThrow(): string {
  try {
    try {
      throw new Error("first");
    } finally {
      throw new Error("second");
    }
  } catch (e) {
    if (e instanceof Error) {
      return e.message;
    }
    return "other";
  }
}

function loops(): string {
  let log = "";
  for (let i = 0; i < 3; i++) {
    try {
      if (i === 0) {
        continue;
      }
      if (i === 2) {
        break;
      }
      log += `[body${i}]`;
    } finally {
      log += `[fin${i}]`;
    }
  }
  return log;
}

function catchThenFinally(): string {
  let log = "";
  try {
    try {
      throw new Error("a");
    } catch {
      log += "[catch]";
      throw new Error("b");
    } finally {
      log += "[finally]";
    }
  } catch (e) {
    if (e instanceof Error) {
      log += e.message;
    }
  }
  return log;
}

export function main(): void {
  print(keep());
  print(replaceReturn());
  print(replaceThrow());
  print(loops());
  print(catchThenFinally());
}
