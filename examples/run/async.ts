// An async `main`. The runner starts it as a call task, pumps the
// instance once, and reads the result. This runner grants no async host
// functions, so the task settles in that one pump.

async function message(): Promise<string> {
  return "hello from async main";
}

export async function main(): Promise<void> {
  print(await message());
}
