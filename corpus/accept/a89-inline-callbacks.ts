// corpus: accept/a89-inline-callbacks
// purpose: Arrow literal callbacks of forEach, map, filter, and reduce: order, captures, index, exceptions, and the forms that keep the helper.
// rule: C6
// phase: P6
// tsc: accepts
// js-comparable: yes
let log = "";

function note(s: string): number {
  log += s;
  return log.length;
}

function source(): number[] {
  note("a");
  return [3, -0, 0.5, 7, 0 / 0];
}

function start(): number {
  note("i");
  return 100;
}

function order(): string {
  log = "";
  const total = source().reduce((acc: number, v: number, i: number): number => acc + i, start());
  return `${total} ${log}`;
}

function values(): string {
  const xs = source();
  const k = 2;
  const doubled = xs.map((v: number): number => v * k);
  const indexed = xs.map((v: number, i: number): string => `${i}:${v}`);
  const kept = xs.filter((v: number, i: number): boolean => v === v && i % 2 === 0);
  const sum = xs.reduce((acc: number, v: number): number => (v === v ? acc + v : acc), 0);
  let seen = "";
  xs.forEach((v: number, i: number): void => {
    seen += `${i}`;
  });
  return `${doubled.join(",")} ${indexed.join(",")} ${kept.join(",")} ${sum} ${seen} ${1 / doubled[1]}`;
}

function captures(): string {
  let scale = 1;
  const xs: number[] = [1, 2, 3];
  const bump = (): number => {
    scale++;
    return scale;
  };
  const ys = xs.map((v: number): number => v * scale + bump());
  return `${ys.join(",")} ${scale}`;
}

class Point {
  x: number;
  constructor(x: number) {
    this.x = x;
  }
}

function fields(): string {
  const ps: Point[] = [new Point(1), new Point(2)];
  const names: string[] = ["p", "q"];
  const xs = ps.map((p: Point, i: number): number => p.x + names[i].length + ps.length);
  const ok = names.filter((s: string): boolean => s.length > 0 && s !== "q");
  return `${xs.join(",")} ${ok.join(",")}`;
}

function thrown(): string {
  const xs: number[] = [1, 2, 3];
  let reached = 0;
  try {
    xs.forEach((v: number): void => {
      reached = v;
      if (v === 2) {
        throw new Error("stop");
      }
    });
  } catch (e) {
    if (e instanceof Error) {
      return `${reached} ${e.message}`;
    }
  }
  return "none";
}

function helpers(): string {
  const xs: number[] = [4, 5];
  const f = (v: number): number => v + 1;
  const named = xs.map(f);
  const block = xs.map((v: number): number => {
    return v + 2;
  });
  const nested = xs.map((v: number): number => [v].map((w: number): number => w + 3)[0]);
  return `${named.join(",")} ${block.join(",")} ${nested.join(",")}`;
}

export function main(): void {
  print(order());
  print(values());
  print(captures());
  print(fields());
  print(thrown());
  print(helpers());
}
