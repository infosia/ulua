export function setup(): number {
  print("module without main");
  return 1;
}

const value: number = setup();
