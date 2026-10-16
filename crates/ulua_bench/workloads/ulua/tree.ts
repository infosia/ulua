class Tree {
  left: Tree | null; right: Tree | null;
  constructor(depth: number) {
    this.left = null; this.right = null;
    if (depth > 0) { this.left = new Tree(depth - 1); this.right = new Tree(depth - 1); }
  }
}
function walk(node: Tree | null): number {
  if (node === null) { return 0; }
  return (1 + walk(node.left) + walk(node.right)) % 4294967296;
}
export function main(): number {
  let sum = 0;
  for (let i = 0; i < 12; i++) { const root = new Tree(14); sum = (sum + walk(root)) % 4294967296; }
  return sum;
}
