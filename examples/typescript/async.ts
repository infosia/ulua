// Async functions. `async`, `await`, and `Promise.all` work as in
// JavaScript, and the steps run in the same order as in node. The
// difference is who runs the program: there is no event loop and no
// timer. The host calls a function, and the script runs until all the
// work that it can do now is done.

async function double(n: number): Promise<number> {
  print(`double ${n}`);
  return n * 2;
}

async function task(name: string, n: number): Promise<number> {
  // The function runs up to its first `await` at once.
  print(`${name} starts`);
  const d = await double(n);
  // The rest runs later, after the other work that is ready.
  print(`${name} continues with ${d}`);
  return d + 1;
}

async function sum(values: number[]): Promise<number> {
  // `Promise.all` waits for all promises and keeps their order.
  const doubled = await Promise.all(values.map((v: number): Promise<number> => double(v)));
  return doubled.reduce((s: number, v: number): number => s + v, 0);
}

export async function main(): Promise<void> {
  // Both tasks start before either one continues, as in node.
  const a = task("a", 1);
  const b = task("b", 2);
  print("both started");
  print(`results ${await a} ${await b}`);
  print(`total ${await sum([1, 2, 3])}`);

  // A promise must be used: stored, passed, returned, or awaited. A call
  // such as `task("c", 3);` on its own line does not compile, because a
  // forgotten promise is a common bug.

  // Ulua has no `Promise.resolve`, `new Promise`, or `setTimeout`. An
  // async function makes each promise. For work that waits on the
  // outside world, the host gives the script an async function; see
  // `examples/async/`.
}
