// Host functions are capabilities. A script can call only the functions
// that the host grants to `ulua_compile`.
//
// This host grants `damage` and `log`. It writes their declarations to
// `game.d.ts` with `ulua_declarations`, so `tsc` and the editor know
// their types. A call to a function that the host did not grant is a
// compile error.

// A module-level binding keeps its value between two calls of the
// exports, for the life of the instance.
let health: number = 100;

// The host calls `hit(power)`. The script asks the host for the damage,
// then reports through the host log. After the host revokes the grant of
// `damage`, the next call of `hit` stops with `host:revoked`, and no
// `catch` in the script sees that stop.
export function hit(power: number): number {
  const amount = damage(power);
  health -= amount;
  log(`hit for ${amount}, health ${health}`);
  return health;
}

// `heal` returns `false` and calls no host function when the health is
// full.
export function heal(): boolean {
  if (health >= 100) {
    return false;
  }
  health = 100;
  log("healed");
  return true;
}
