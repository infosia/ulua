# Benchmarks — contract

Status: Rev 0, 2026-10-16 (P6). The contract follows the benchmark
contract of the owner's prior project: its subjects, its timing method,
its workload set, and its boundary-price measurement (`plan.md` P6).
Each change from that contract is stated with the Ulua rule that forces
it.

The suite is a published comparison, not a gate. The performance gate
of §6 is the gate. The code lives in `crates/ulua_bench`.

## 1. Reporting rules

1. Report measured numbers, whatever they are. No subject is tuned to
   flatter Ulua. No baseline is weakened.
2. The C baseline is 1.00×. Each other subject is reported as the ratio
   of its median to the C median, and its absolute median.
3. Every subject computes the same workload and gives the same integer
   checksum. The runner withholds the timings of a workload if any
   measured checksum differs.
4. The suite names and cites only the runtimes that it compares, by
   upstream URL (CLAUDE.md hygiene). The workloads are public
   algorithms written fresh here.

## 2. Subjects

| Subject | What it is | Timed by |
|---|---|---|
| C | hand-written C, `clang -O2 -fwrapv -ffp-contract=off` | itself (monotonic clock) |
| ulua | the Ulua workload through the product C ABI, Release library | the runner, around `ulua_call` of the exported workload function |
| luau | the same algorithm written by hand in Luau, on the pinned VM with the production compile options (§5.19 of `pipeline.md`: O1), no interrupt callback | the runner, around `lua_pcall` |
| LuaJIT | [luajit.org](https://luajit.org) 2.1 | itself (`os.clock`) |
| V8 | Node.js ([nodejs.org](https://nodejs.org)) | itself |
| JSC | JavaScriptCore ([webkit.org](https://webkit.org)), when present | itself |

- Ulua has no clock primitive, so the runner times the `ulua` and
  `luau` subjects from outside. The others time themselves and print
  `<checksum> <median_seconds>` and every sample.
- The `luau` subject is new in this contract. The ratio `ulua / luau`
  prices the lowering and the limits (interrupt points, step and
  allocation charges, the helpers) against the same VM. The `luau`
  host links the pinned VM directly. It is a benchmark tool, not a
  public API (invariant 6).
- The `ulua` subject runs with `max_steps` and `max_memory_bytes` set by
  the runner to values that no workload reaches. The record states the
  values. The step and allocation charges stay in the measured time:
  they are a cost of the product.

## 3. Timing method

1. Each subject: at least 11 timed runs; report the median; record the
   minimum, the maximum, and every sample.
2. The noise gate reads the interquartile range. If the range is wider
   than 15% of the median, the runner withholds the timing of that
   subject. The quartile at `p` of sorted samples `x[0..n]` is at
   position `(n - 1) * p`, by linear interpolation.
3. Warm-up is a time floor: at least 200 ms of measured warm-up
   execution and at least 3 iterations. Each subject reports its
   measured warm-up. The runner rejects a subject below the floor. A C
   harness writes its warm-up result to a `volatile` sink, so the
   optimizer cannot delete the warm-up loop.
4. Each `(workload, subject)` measurement runs in a fresh process.
5. Only the workload execution is timed: not process start, not
   compilation, not instance creation, not output.
6. `--only <id>` prints the report to stdout and writes no record. It
   says on stderr that it wrote no file. Only a run over every workload
   writes `results.json` and the README table. `--only` gives the same
   figure as the full run for the same cell.
7. One machine, one session. The record names the machine (host, CPU,
   power source) and the version of each runtime. An absent runtime is
   `-`, never zero, never a silent skip.

## 4. Workloads

The ten workloads of the prior contract, with one change: Ulua rejects
the bitwise operators (`collisions.md` C4 rule 7), so no subject uses
i32 wrapping. Each checksum is an exact integer below 2^53 in every
subject. A running sum is reduced with `% 4294967296` (unsigned 32-bit)
in every subject; C uses `uint64_t` for it. The shared LCG is
`state = (state * 1664525 + 1013904223) % 4294967296`; the product
stays below 2^53.

| Id | Workload | Checksum |
|---|---|---|
| fib-recursive | naive `fib(n)`, n = 31 | `fib(31)` = 1346269 |
| fib-loop | iterative `fib` in a loop, accumulated | the reduced sum |
| mandelbrot | escape count over an N×N grid, test `x²+y² ≥ 4`, cap 255 | sum of escape counts |
| primes | count primes up to N by trial division | the count |
| sort | quicksort of N LCG numbers, written by hand | reduced sum of sampled indices |
| tree | build and walk balanced binary trees of class instances to a fixed depth | node-visit count |
| queen | N-queens solution count | the count |
| particles | M records, K steps of `pos += vel * dt` | reduced sum of the quantized positions |
| callbacks | `map`, `filter`, `reduce` over N LCG numbers, K rounds, each callback takes the index | reduced sum of each round's `reduce` |
| collect | build a graph of mixed-size records and strings, drop part, keep the rest | reduced checksum over the kept graph |

- The implementer pins N, M, K, and each checksum formula, the same in
  every subject, and records them in the README of the crate. The C
  baseline runs in about 10 to 500 ms per iteration (a target).
- `tree` and `collect` allocate through each runtime's own memory
  manager. No subject forces a collection, and no subject frees by
  hand except C. This changes the prior contract: Ulua has no
  collection API and no explicit free.
- `callbacks` measures the idiomatic callback spelling against a
  hand-written loop: C, Luau, and LuaJIT write loops; Ulua and the
  JavaScript runtimes call the array methods. The published row says
  this.
- The Ulua workloads are accepted programs: they type-check under
  `tsc` with the prelude, and each one is compiled at the default
  compile limits.

## 5. Boundary price (P6.2)

Two boundaries, each a loop of 1,000 calls, measured from inside and
decomposed:

| Boundary | Region | Floor |
|---|---|---|
| host into script | 1,000 `ulua_call` of an exported function `f(a: number, b: number): number` that returns `a + b` | 1,000 `lua_pcall` of the same Luau function on the pinned VM |
| script into host | one `ulua_call` of a script loop of 1,000 calls of a granted host function `(number, number) => number` | the same loop that calls a plain `lua_CFunction` |

- Each variant runs in a fresh process, with the timing method of §3
  and a clock whose quantum is at most 1% of the median region span
  (the record states the quantum).
- The record states ns per call for each boundary and the share of
  each layer: argument conversion, the shim and the trampoline, the
  step and interrupt accounting, and the rest. It names a shrink
  candidate with its measured share, or states that the call is the
  floor.
- No production change in P6.2. A shrink lands as a later slice, on
  this evidence.

## 6. Performance gate (P6.3)

A test in `gate.sh timing` asserts the ratio `ulua / luau` of a fixed
set of workloads at reduced sizes against a bound from the P6.1
measurement (the measured ratio times 1.25). It asserts ratios, not
budgets (principle 15). The set and the bound are fixed at P6.3 from
the P6.1 record.

## 7. Layout

```
crates/ulua_bench/              the benchmark crate (bin `cross-language`, bin `boundary`)
  workloads/ulua/<id>.ts        one exported workload function that returns the checksum
  workloads/luau/<id>.luau      the hand-written Luau
  workloads/c/<id>.c            baseline and self-timing harness
  workloads/js/<id>.js          run under node and jsc
  workloads/lua/<id>.lua        run under luajit
  results.json                  captured medians and samples
  README.md                     the generated table (C = 1.00×), machine and versions
```
