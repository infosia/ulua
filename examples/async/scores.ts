// An async host function. `fetchScore` is a grant with a `start`
// function: a script call returns a promise and starts a host request.
// The host completes each request later, then pumps the instance, and
// the awaiting functions resume. Nothing runs between two host calls.
//
// The host writes the declaration `fetchScore(p0: number):
// Promise<number>` to `scores.d.ts` with `ulua_declarations`.

// `total` starts one request per id at once and waits for all of them.
// `Promise.all` keeps the order of the ids, as in JavaScript.
async function total(ids: number[]): Promise<number> {
  const scores = await Promise.all(ids.map((id: number): Promise<number> => fetchScore(id)));
  return scores.reduce((sum: number, s: number): number => sum + s, 0);
}

// The host starts `report` as a call task (`ulua_call_start`). The task
// stays pending until the host completes the three requests and pumps.
export async function report(): Promise<number> {
  print("requesting scores");
  try {
    const sum = await total([1, 2, 3]);
    print(`total ${sum}`);
    return sum;
  } finally {
    // `finally` runs when the task settles, as in JavaScript.
    print("report done");
  }
}
