// Scripts are hostile. The host sets the limits of each instance
// (`ulua_limits`), and a stop leaves the instance usable: the host can
// call another export after it.
//
// Each export below stops in a different way. No `catch` and no
// `finally` in a script sees a stop.

const xs: number[] = [1, 2, 3];

// A loop that does not end. Each loop charges steps, so the run stops
// with `limit:steps` when it reaches `max_steps`.
export function spin(): void {
  let i: number = 0;
  while (true) {
    i++;
  }
}

// A string that doubles without end. The allocator charges each
// allocation, so the run stops with `limit:memory` at `max_memory_bytes`.
export function grow(): void {
  let s: string = "x";
  while (true) {
    s = s + s;
  }
}

// A read outside the array. JavaScript gives `undefined`; Ulua stops the
// run with `trap:index-out-of-range`, because the value would be outside
// the type `number`.
export function outOfRange(): number {
  return xs[10];
}

// After the three stops, this call still runs and returns 3.
export function ok(): number {
  return xs.length;
}
