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
| Invariant 3: TS2367 and TS2845 at an accepted equality expression are outside the rule | 2026-10-12 | Owner decision after a measurement round (`tracking/equality-measurements.md`): type-based equality; 30 newly accepted programs equal `node`, 20 show only these codes. |
| Invariant 3: TS2322 and TS2345 that name `never` after an accepted equality are outside the rule | 2026-10-12 | Owner decision after the P2.3 round 3 review: a contradictory accepted equality makes `tsc` narrow to `never`, and a later write or copy then gives TS2322 or TS2345 (`if (k === 1) { if (k === 2) { k *= 2; } }`). |
| Invariant 3: TS2339 that names `never` joins the `never` exception | 2026-10-12 | Same class as the owner's decision for TS2322 and TS2345: after a contradictory accepted equality, `s.length` gives TS2339 on `never` (reachable since P2.4 opened string members). |
| Invariant 3: the `never` exception also covers the narrowing after an assignment of `null` | 2026-10-12 | P2.5 Phase Review round 1 (M2): after `let s: string \| null = null;`, `s?.length` gives TS2339 on `never` with no equality; the program agrees with `node`. The compiler checks types without the `tsc` narrowing (C16). Same class as the owner's decision for the `never` exception; the owner confirmed it. `a42` pins it. |
| Invariant 3: the `never` exception names the null test (C2) beside the equality | 2026-10-12 | P2.5 Phase Review round 4: `if (k == null) { out += k?.v ?? 0; }` gives TS2339 that names `never` after a C2 null test; C7 leaves null operands to C2. |
| Invariant 3: TS2321 is a `tsc` resource limit | 2026-10-12 | P2.5 Phase Review round 7: two rings of recursive aliases with coprime lengths (25 and 26) give TS2321 "Excessive stack depth comparing types"; at 20 and 21 `tsc` is clean, and equal lengths pass. The program agrees with `node`. Same class as TS2563, which the owner placed outside the rule. |
| Invariant 3: every diagnostic that names the type `never` is outside the rule | 2026-10-13 | P2.5 Phase Review round 11: after `let k: F \| null = null;`, `if (k != null) { k(1); }` gives TS2349 and `for (const x of xs)` gives TS2488 on `never`; 561 of 10,006 accepted random programs, each equal to `node`. Same class as the owner's `never` exception (TS2322, TS2345, TS2339); the rule names the class, not each code. |
| Principle 15: a gate assertion never depends on wall-clock time or machine load | 2026-10-13 | P2.5 Phase Review round 12: `p25_growth.rs` asserted a 12 s Debug budget; it measured 9.2 to 9.4 s alone and 14.3 to 19.4 s with 3 or 4 concurrent copies (33 of 33 failed), while its 4x ratio stayed at about 4.06. One `gate.sh full` run failed this way. |
| Invariant 3: `tsc` checks with the prelude and the host declarations of the grants | 2026-10-13 | Owner decision at the P3 start, after a comparison with the owner's prior project, which generates declaration files from the host interface: the grant list is the only record of a host function, and the library writes its `.d.ts` (D20 revised). |
