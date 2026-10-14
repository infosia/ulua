// `fetchScore` is an async host function: a call starts a request, and
// the host completes it later and pumps the instance.
async function total(ids: number[]): Promise<number> {
  const scores = await Promise.all(ids.map((id: number): Promise<number> => fetchScore(id)));
  return scores.reduce((sum: number, s: number): number => sum + s, 0);
}

export async function report(): Promise<number> {
  print("requesting scores");
  try {
    const sum = await total([1, 2, 3]);
    print(`total ${sum}`);
    return sum;
  } finally {
    print("report done");
  }
}
