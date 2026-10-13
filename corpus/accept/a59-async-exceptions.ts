// corpus: accept/a59-async-exceptions
// purpose: A throw in an async body rejects its promise, and await throws the reason again.
// rule: C19
// phase: P4.1
// tsc: accepts
// js-comparable: yes
async function fail(message: string): Promise<number> {
  print(`fail ${message}`);
  throw new RangeError(message);
}
async function guarded(): Promise<string> {
  try {
    const n = await fail("one");
    return `value ${n}`;
  } catch (e) {
    if (e instanceof RangeError) {
      return `caught ${e.name}: ${e.message}`;
    }
    throw e;
  } finally {
    print("guarded finally");
  }
}
async function cleanup(): Promise<number> {
  let n: number = 0;
  try {
    n = await fail("two");
  } catch (e) {
    n = -1;
  } finally {
    n = n * 10;
    print(`cleanup finally ${n}`);
  }
  return n;
}
async function rethrow(): Promise<number> {
  try {
    return await fail("three");
  } finally {
    print("rethrow finally");
  }
}
export async function main(): Promise<void> {
  print(await guarded());
  print(`${await cleanup()}`);
  try {
    print(`${await rethrow()}`);
  } catch (e) {
    if (e instanceof Error) {
      print(`main caught ${e.message}`);
    }
  }
  print("done");
}
