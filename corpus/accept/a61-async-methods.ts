// corpus: accept/a61-async-methods
// purpose: Async methods use this, and async arrows capture bindings.
// rule: C19
// phase: P4.1
// tsc: accepts
// js-comparable: yes
class Counter {
  count: number = 0;
  async add(n: number): Promise<number> {
    this.count += n;
    await this.tick();
    this.count += n;
    return this.count;
  }
  async tick(): Promise<void> {
    print(`tick ${this.count}`);
  }
}
export async function main(): Promise<void> {
  const c = new Counter();
  let log: string = "";
  const step = async (name: string): Promise<number> => {
    log += name;
    const v = await c.add(1);
    log += `${v}`;
    return v;
  };
  const x = step("x");
  const y = step("y");
  print(`${await x} ${await y} ${c.count} ${log}`);
}
