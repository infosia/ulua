// corpus: accept/a05-bindings
// purpose: Declares, assigns, and shadows block-scoped bindings.
// rule: C1
// phase: P1
// tsc: accepts
export function main(): void {
  let count = 1;
  const step = 2;
  count = count + step;
  count += step;
  {
    const count = 100;
    print(`${count}`);
  }
  let flag: boolean = true;
  flag = !flag;
  print(`${count} ${flag}`);
}
