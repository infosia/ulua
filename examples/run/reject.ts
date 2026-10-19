// A rejected script. `print` takes a string, and `1` is a number, so
// `tsc` and the Ulua compiler both reject the call. The runner prints the
// diagnostic (`error C1 at <line>:<column>: ...`) and exits with status 1.

export function main(): void {
  print(1);
}
