# P7 standard library

Status: PLANNED 2026-10-17. Contract: `blocks/stdlib.md` Rev 0.

## Slices

`blocks/stdlib.md` §3. P7.0 (measurement round) opens first.

## Measurements (planning probe)

Pinned VM (`luau` CLI at `luau-revision.txt`) and `node` v24.18.0.
A probe, not the M1 measurement: one input per row.

| Form | Luau | `node` |
|---|---|---|
| `round(2.5)`, `round(-2.5)` | 3, -3 | 3, -2 |
| `round(0.49999999999999994)`, `round(-0.4)` | 0, -0 | 0, -0 |
| `max(NaN, 1)`, `max(1, NaN)` | NaN, 1 | NaN, NaN |
| `max(0, -0)`, `max(-0, 0)` | 0, -0 | 0, 0 |
| `min(0, -0)`, `min(-0, 0)` | 0, -0 | -0, -0 |
| `sign(-0)`, `sign(NaN)` | 0, 0 | -0, NaN |
| `abs(-0)`, `floor(-0.5)`, `ceil(-0.5)` | 0, -1, -0 | 0, -1, -0 |
| `1 ** NaN`, `(-1) ** Infinity`, `NaN ** 0` (Luau `^`) | 1, 1, 1 | NaN, NaN, 1 |
| `log2(8)` (`math.log(8, 2)`), `log10(1000)`, `log(-0)` | 3, 3, -inf | 3, 3, -Infinity |
| `atan2(-0, -1)`, `sqrt(-0)`, `sqrt(-1)` | -π, -0, NaN | -π, -0, NaN |
| `toFixed`: 2.5 at 0, 0.5 at 0, 1.005 at 2, 0.25 at 1 (`string.format("%.nf")`) | 2, 0, 1.00, 0.2 | 3, 1, 1.00, 0.3 |

Absent from the Luau `math` library: `trunc`, `cbrt`, `hypot`, `log2`,
`expm1`, `log1p`. Present: `sinh` (and the other members of the probe).

Consequence: `round`, `max`, `min`, `sign`, and `pow` need form 2 or 3
of `stdlib.md` §0.2; `toFixed` needs a helper (`%.nf` rounds the
binary value half to even, `toFixed` picks the larger `n` on a tie).
