async function message(): Promise<string> {
  return "hello from async main";
}

export async function main(): Promise<void> {
  print(await message());
}
