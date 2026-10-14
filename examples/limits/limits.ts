// Scripts are hostile: the host sets run limits, and a stop leaves the
// instance usable.
const xs: number[] = [1, 2, 3];

export function spin(): void {
  let i: number = 0;
  while (true) {
    i++;
  }
}

export function grow(): void {
  let s: string = "x";
  while (true) {
    s = s + s;
  }
}

export function outOfRange(): number {
  return xs[10];
}

export function ok(): number {
  return xs.length;
}
