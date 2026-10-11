# Rules history

`CLAUDE.md` states each rule without a date or an origin. This file
holds, per rule, the decision date and the evidence. Nothing here is a
current rule.

| Rule (CLAUDE.md) | Decided | Evidence and origin |
|---|---|---|
| Invariant 1: embedded with a C ABI | 2026-10-11 | Owner decision. A prior proof of concept with the same shape established the Luau backend and the C boundary. |
| Invariant 2: scripts are hostile | 2026-10-11 | Owner decision. The host runs content that it did not write. |
| Invariant 4: native Luau representation | 2026-10-11 | Owner decision. In the prior proof of concept, sentinels for absence, UTF-16 strings in Luau strings, and explicit-length arrays required a helper for most operations and a conversion at each host boundary. |
| Invariant 5: accept only agreeing forms; traps where node leaves the static type | 2026-10-11 | Owner decision. With invariant 4, every accept entry is comparable with node, so node detects each wrongly accepted form. Out-of-range reads and holes are where the TypeScript type system is unsound. |
| Invariant 6: emit typed Luau source; no bytecode | 2026-10-11 | Owner decision on typed output. The prior proof of concept removed a direct bytecode emitter: each emitter or verifier gap was a VM memory-safety defect for hostile input. |
| Oracle and witnesses: `luau-analyze` is a witness | 2026-10-11 | Owner decision on typed output (D4). |
| Roles: implementation agent is a Claude Opus subagent | 2026-10-11 | Owner decision. The workflow follows a prior project of the owner. Replaced the same day (next row). |
| Roles: implementation agent is Codex through the Codex plugin | 2026-10-11 | Owner decision at the start of P1. |
| Core principles 1–16 | 2026-10-11 | Adopted from a prior project of the owner, with the two execution tiers replaced by the four witnesses. |
| Code conventions: 1,000 lines per source file | 2026-10-11 | Smaller files keep implementation handoffs small. |
| Severity follows invariant 2 | 2026-10-11 | Hostile scripts make any script input a realistic reach. |
| Environment: `LUAU_SOURCE_DIR` | 2026-10-11 | A local checkout path is machine-specific and stays out of the repository. |
| Invariant 5: JavaScript agreement is the default; a divergence is decided and named | 2026-10-11 | Owner decision in the P1 Phase Review: exact `node` agreement for every accepted form is too strict. The level follows the owner's prior project: `node` is a divergence detector, and a `js-comparable: no` entry cites its collision. |
| Invariant 3: `tsc` resource limits are outside the rule | 2026-10-11 | Owner decision in P1 Phase Review round 5: 1,000 `if` statements in one function (about 14 KB) give TS2563; a 64 KB loop input overflows the `tsc` stack. Both are inside `max_source_bytes`. |
