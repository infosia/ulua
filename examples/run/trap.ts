// A trap. The read `values[2]` is outside the array, so the run stops
// after the first line. The runner prints the stop code and exits with
// status 2. node prints `undefined` in place of the stop.

export function main(): void {
  const values: number[] = [1];
  print("before the trap");
  print(`${values[2]}`);
}
