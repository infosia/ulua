// The general runner (`examples/run/host.c`) compiles a script with no
// grants and calls its export `main`. It prints each line that the
// script writes.

export function main(): void {
  print("hello from the runner");
}
