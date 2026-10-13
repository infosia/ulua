# P3 host interop

Status: open. Contracts: `abi.md` Rev 5, `collisions.md` Rev 41 (C18),
`surface.md` Rev 23, `pipeline.md` Rev 28 (§4.10, §8), `corpus.md` Rev 9
(§3 `grants`, §7 fixtures). Decisions: D20 (revised: the library
writes the host declarations; the script declares nothing), D21, D22
(`plan.md` §8).

## Entries

| Phase | Family | Entries |
|---|---|---|
| P3 | Host functions and capabilities | `a55`, `a56`, `a57`; `r126`, `r128`, `r130` to `r133`; retired `r127`, `r129` |

## Exit checklist (P3)

- [ ] abi.md Rev 5 (§1.2, §1.7, §3 host codes, §4.3 to §4.6 string
  values, §6.2 capabilities, §6.3 `ulua_declarations`, §7 functions,
  §8 link tests); collisions.md C18; surface.md Rev 23;
  pipeline.md §4.10 and §8 `grants`; corpus.md §3 and §7 implemented.
  `include/ulua.h` declares every new item.
- [ ] P3 entries pass in full; every earlier entry keeps passing.
- [ ] `ulua_declarations`: the exact text for each type, grant order,
  an empty list, the length query, a small capacity, duplicate names,
  a revoked grant; a Rust test compares its text for the fixtures with
  the text that `tools/witness/corpus.mjs` writes (two derivations,
  CLAUDE.md principle 9).
- [ ] Direct tests (one per public function, CLAUDE.md principle 1):
  each invalid capability descriptor; duplicate and revoked grants at
  compile, an unused grant, a shadowed grant, an unknown name (C18); each value type in
  both directions, NaN, -0, an empty string, non-ASCII text, a 1 MiB
  string result; each malformed result (wrong type, a non-zero unused
  field, boolean 2, invalid UTF-8, null data with a length) gives
  `host:result`; a host error gives `host:capability` inside `try`,
  `catch`, and `finally` and runs no `catch` and no `finally`; a
  revoked capability gives `host:revoked` and the host is not called;
  each function of §7 called from a host callback gives `ULUA_BUSY`;
  a capability freed while a module and an instance hold it; a host
  string result over `max_memory_bytes` gives `limit:memory`; one step
  per host call; `ulua_call` with invalid UTF-8 and with string
  results; the instance stays usable after each stop.
- [ ] A panic in the Rust code around a host call gives
  `ULUA_INTERNAL` and poisons the context (carry-over M2,
  `tracking/p05-measurements.md`).
- [ ] Link tests: C11 and C++17 programs with `-Wall -Wextra
  -Wpedantic -Werror`, against the static and the dynamic library;
  the exported symbols of the dynamic library equal abi.md §7.
- [ ] Limits: host declarations at and past the local budget (spill)
  and the upvalue budget; growth shapes of host calls inside §3.10.
- [ ] A random check of host calls against `node` (fixtures), `tsc`,
  and `luau-analyze -j1`.
- [ ] Phase Review with no open CRITICAL or MAJOR finding.

Source: the owner's prior projects. The declarations follow the
project that generates a `.d.ts` from the host interface; the
capability model (compile-time grants, revocation checked at each
call, host failures stop the run, the `ulua_value` set of number,
boolean, string, and void) follows the other. Changes: the grant list
is the source of the declarations; an unused grant is valid; an
export can take and give strings.

## Contract pin

At `5b29edf` (first form of D20), `tools/witness/luau.mjs`
(`PHASE = "P3"`) gave: `a55`, `a56` rejected `unsupported`; `r126` to
`r129`, `r131`, `r132` `unsupported`; `r130` C11.

At `2e80285` (D20 revised), built in a separate worktree: `a55`,
`a56`, `a57` rejected C11; `r126`, `r128`, `r130` C11; `r131` C2;
`r132`, `r133` `unsupported`; want C18 for each reject. Every earlier
entry passes.

## Measurements

Implementation record (Codex; three runs stopped with a model capacity
error, so the primary agent read the logs and the tree):

- `sh tools/gate.sh full` passes: `cargo-test` 201 s, `luau-witnesses`
  51 s (175 entries in P3 or earlier, 66 generated modules, 14 stress
  modules); hygiene passes.
- New test targets (Debug): `p3_links` 7.2 s (C11 and C++17, static
  and dynamic, exact dynamic symbol set); `p3_random` 7.5 s (192
  fixed-seed programs against `node`, `tsc`, `luau-analyze -j1`);
  `p3_growth` 10.7 s (compiler ratio at most 8 for 4 times the
  source); `p3_host_names` (the name list against the ES2022 library
  files and the prelude at test time); `p3_declarations` (the ABI text
  against the `node` fixture format); `p3_limits` (spill at the local
  budget, upvalues).
- `measurements` feature: 187 growth shapes; 4 to 16 KiB largest
  ratio 8.47 (bound 12); ceiling: `dense-assign` 0.326 s, largest
  ceiling ratio 31.2 (bound 50); P3 shapes `P3-protected` 24.7,
  `P3-arrows` 2.5, `P3-declarations` 1.1. Cost 649.9 s Debug.

### P3 Phase Review round 1

Reviewer: fresh agent, diff `1fba8ff..282d71f`. 0 CRITICAL, 0 MAJOR, 6 MINOR. Held: 120 hand probes and 150 random programs against `node`, `tsc` with the generated declarations, and `luau-analyze -j1` (0 differences outside C5); 40 value-escape forms rejected; about 30 shadowing forms; 18 stop probes (host error, malformed result, revocation, memory) run no catch and no finally; about 60 ABI probes (results, string sizes, BUSY from all 14 context functions in a callback, free and revoke, 25 invalid descriptors, compile grants, `ulua_declarations` text and edge cases); the name list equals `tsc` `getSymbolsInScope` over ES2022 and the prelude (62 names); the dynamic library exports exactly the 18 functions of §7.

| Finding | Severity | Resolution |
|---|---|---|
| 8,100 called grants (63,722 B) compile, and `ulua_instance_new` gives `limit:stack` each time: one chunk argument per grant reaches `LUAI_MAXCSTACK` (8,000); 7,990 run | MINOR | pipeline.md Rev 29 §4.10: one grant table as one chunk argument |
| Library names outside the surface (`Number("1")`, `JSON.stringify`, `parseInt`, `Date.now()`, `globalThis`) give C18; C18.3 names only unknown names | MINOR | C18.3: these give C11; the checker reads the library name list |
| `namespace N {}` and `module M {}` without `declare` give C18 | MINOR | C18.3: C11 |
| A host call costs 2 steps (the Luau call instruction and the host charge); abi.md §6.2.8 said one | MINOR | abi.md §6.2.8 states both; a test measures with the interrupt on |
| The unused-field check accepts `number = -0.0` (§4.1: zero) | MINOR | Fix: compare the bits |
| `string_exports_compile` repeats one assertion; `new hostAdd(1, 2)` says "Unknown identifier" | MINOR | Fix |

## Failures before the fix

## Gaps

- Contract gap found in implementation: ES2022 does not reserve `eval` and `arguments`. A grant `arguments` called inside a function gives `tsc` TS2349 and a `node` TypeError. abi.md §6.2.1 now rejects both names.
- Contract gap found in implementation: §6.2.1 allowed ES2022 global value names that collisions.md does not accept (`Number`, `Math`). Their `ulua_declarations` text gives `tsc` TS2300 with the library. §6.2.1 now rejects every global value name of the ES2022 library files and the prelude.
