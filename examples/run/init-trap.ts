// A trap in the module initializer. `setup` runs when the runner makes
// the instance, and `values[2]` is outside the array, so the instance is
// not made. The runner prints the stop code and exits with status 2.

export function setup(): number {
  const values: number[] = [1];
  return values[2];
}

const value: number = setup();
