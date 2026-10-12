// corpus: accept/a14-nullable
// purpose: Tests, narrows, and defaults nullable values.
// rule: C2, C7, C16
// phase: P2.5
// tsc: accepts
// js-comparable: yes
type Node = { value: number; next: Node | null };

function find(head: Node | null, value: number): Node | null {
  let current: Node | null = head;
  while (current != null) {
    if (current.value === value) {
      return current;
    }
    current = current.next;
  }
  return null;
}

export function main(): void {
  const list: Node = { value: 1, next: { value: 2, next: null } };
  const hit = find(list, 2);
  const miss = find(list, 9);
  print(`${hit != null} ${miss == null}`);
  print(`${hit?.value ?? -1} ${miss?.value ?? -1}`);
  const label: string | null = null;
  print(label ?? "none");
  const flag: boolean | null = pick(false);
  print(`${flag ?? true}`);
}

function pick(v: boolean): boolean | null {
  return v;
}
