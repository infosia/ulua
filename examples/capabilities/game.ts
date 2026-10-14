// The host grants `damage` and `log` at compile time. The host writes
// their declarations (`game.d.ts`) with ulua_declarations.
let health: number = 100;

export function hit(power: number): number {
  const amount = damage(power);
  health -= amount;
  log(`hit for ${amount}, health ${health}`);
  return health;
}

export function heal(): boolean {
  if (health >= 100) {
    return false;
  }
  health = 100;
  log("healed");
  return true;
}
