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

At the P3 contract commit, `tools/witness/luau.mjs` (`PHASE = "P3"`)
gives: `a55`, `a56` status rejected, code `unsupported`; `r126` to
`r129`, `r131`, `r132` code `unsupported`; `r130` code C11. Every
earlier entry passes.

## Measurements

## Failures before the fix

## Gaps
