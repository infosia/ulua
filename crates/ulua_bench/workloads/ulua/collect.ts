class Cell {
  id: number; text: string; values: number[]; next: Cell | null;
  constructor(i: number, next: Cell | null) {
    this.id = i; this.text = ""; this.values = []; this.next = next;
    for (let j = 0; j < 1 + i % 31; j++) { this.text += "x"; }
    for (let j = 0; j < 1 + i % 7; j++) { this.values.push(i + j); }
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
    for (let i = 0; i < kept.length; i++) {
      const cell = kept[i]; const next = cell.next;
      sum = (sum + cell.id + cell.text.length) % 4294967296;
      if (next !== null) { sum = (sum + next.id) % 4294967296; }
      for (let j = 0; j < cell.values.length; j++) { sum = (sum + cell.values[j]) % 4294967296; }
    }
  }
  return sum;
}
