class Cell {
  id: number; text: string; values: number[]; next: Cell | null;
  constructor(i: number, next: Cell | null) {
    this.id = i; this.text = ""; this.values = []; this.next = next;
    const textLength = 1 + i % 31;
    for (let j = 0; j < textLength; j++) { this.text += "x"; }
    const valueLength = 1 + i % 7;
    for (let j = 0; j < valueLength; j++) { this.values.push(i + j); }
  }
}
export function main(): number {
  let sum = 0;
  for (let k = 0; k < 4; k++) {
    let all: Cell[] = []; const kept: Cell[] = []; let previous: Cell | null = null;
    for (let i = 0; i < 40000; i++) {
      const cell: Cell = new Cell(i, previous); all.push(cell);
      if (i % 2 === 0) { previous = cell; }
    }
    for (let i = 0; i < 40000; i += 2) { kept.push(all[i]); }
    all = [];
    const keptLength = kept.length;
    for (let i = 0; i < keptLength; i++) {
      const cell = kept[i]; const next = cell.next;
      sum = (sum + cell.id + cell.text.length) % 4294967296;
      if (next !== null) { sum = (sum + next.id) % 4294967296; }
      const valueLength = cell.values.length;
      for (let j = 0; j < valueLength; j++) { sum = (sum + cell.values[j]) % 4294967296; }
    }
  }
  return sum;
}
