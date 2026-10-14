// A first Ulua script. The host calls the export `main`.
type Point = { x: number; y: number };

class Counter {
  count: number = 0;
  add(n: number): number {
    this.count += n;
    return this.count;
  }
}

function lengthSquared(p: Point): number {
  return p.x * p.x + p.y * p.y;
}

export function main(): void {
  print("hello from Ulua");
  const words: string[] = ["alpha", "beta", "gamma"];
  print(words.map((w: string): string => `<${w}>`).join(" "));
  const c = new Counter();
  c.add(2);
  print(`count ${c.add(3)}`);
  print(`length squared ${lengthSquared({ x: 3, y: 4 })}`);
}

export function greet(name: string): string {
  return `hello, ${name}`;
}
