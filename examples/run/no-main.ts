// A module without `main`. The module initializer runs when the runner
// makes the instance, so `setup` prints its line. The runner then finds
// no `main` and exits with status 0.

export function setup(): number {
  print("module without main");
  return 1;
}

const value: number = setup();
