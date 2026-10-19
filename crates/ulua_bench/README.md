# Ulua and Luau benchmark suite

This suite measures the product C ABI against hand-written Luau on the same pinned VM.
The suite is a published comparison. It is not a performance gate.
No product crate depends on this crate.

Source: [Luau](https://luau.org), [upstream repository](https://github.com/luau-lang/luau).
The captured record below states the revision from `luau-revision.txt`.
The `bench` binary measures workloads. The `boundary` binary measures the two C ABI boundaries.

Set `LUAU_SOURCE_DIR` to the pinned checkout. Run from the repository root:

```sh
CARGO_NET_OFFLINE=true cargo run --release -p ulua_bench --bin bench
CARGO_NET_OFFLINE=true cargo run --release -p ulua_bench --bin bench -- --only sort
```

Only a full run with no Git changes under `crates/` writes `results.json` and the marked table below.
The runner checks staged, unstaged, and untracked files before measurement and before publication.
If the check fails, the runner reports that it wrote no file.
Changes outside `crates/` do not block publication.
`--only` prints the table and all raw observations to stdout. It writes no file.
The parent starts a fresh child process for each workload and subject, in sequence.
Compilation, instance creation, function preparation, result checks, and output stay outside each timed region.
Both subjects use `std::time::Instant` around one workload call.
Luau calls `lua_pcall` directly. Ulua calls the product `ulua_call` C ABI.
The native host uses the same compiler defaults as the product: O1, debug level 1, type information level 0.
The native host installs no interrupt callback. The product retains its limits and allocation charges.
Neither subject forces a collection or frees script objects by hand.

Each cell warms up for at least 0.2 seconds of measured execution and at least three iterations.
Each cell then supplies 11 timed samples. Warm-up and timed checksums must all agree.
A checksum mismatch withholds the whole workload. A noisy cell withholds its own timing and the workload ratio.
The noise threshold is IQR greater than 15% of the median.
Quartiles use linear interpolation at `(n - 1) * p`.
The record retains every sample, min, median, max, quartiles, IQR, and measured warm-up.
`median_seconds: null` means that the published cell is withheld. `observed` retains the evidence.
New host IDs use FNV-1a, 64 bits, over the hostname bytes.
The metadata field `host_hash` names this fixed hash. The record omits personal machine names and paths.

## Fixed parameters and checksum formulas

All indices below start at zero, including callback indices. Lua table indices add one at storage access.
All running sums use `% U`, where `U = 4294967296`. All checksums are integers below `U`.
The LCG starts at `state = 1`, then uses `state = (state * 1664525 + 1013904223) % U`.
The largest LCG intermediate is `7149081450614098`, below `2^53`.
No subject uses bitwise operators or i32 wrapping.

| Id | Parameters | Algorithm and checksum |
|---|---|---|
| fib-recursive | N = 31 | Naive recursion; F(0) = 0, F(1) = 1; return F(31) % U = 1346269. |
| fib-loop | N = 1000000; M = 40 | For each i in [0,N), compute F(i % M) by iteration; sum each F modulo U. |
| mandelbrot | N = 400; K = 255 | Grid: cx = col/N*3-2, cy = row/N*2-1. Start z = 0. Update z while norm squared < 4 and count < K. Sum counts modulo U. |
| primes | N = 200000 | Trial divisors d = 2 through floor(sqrt(n)), for n = 2 through N inclusive. Count primes modulo U. |
| sort | N = 100000; M = 97 | Generate N LCG values. Quicksort with the middle element as pivot. Sum sorted a[i] at i = 0,M,2M,... below N, modulo U. |
| tree | Depth = 14; K = 12 | Build K full binary trees of class instances. Depth zero has one node. Sum recursive node visits modulo U; checksum = 393204. |
| queen | N = 11 | Row-first backtracking, columns 0 through N-1. Test earlier columns and diagonals by loops. Count solutions modulo U. |
| particles | M = 20000; K = 200; dt = 0.125 | Initial x = i%100, y = i%71, vx = i%13+1, vy = i%17+1. Update both positions K times. Sum floor(x*8)+floor(y*8) modulo U. |
| callbacks | N = 100000; K = 20 | Generate N LCG values once. Round k maps (v+i+k)%U, filters (v+i)%3 != 0, then reduces (acc+v+i)%U from zero. Sum round results modulo U. Ulua uses methods; Luau uses loops. |
| collect | N = 40000; K = 4; string M = 31; array M = 7 | Each round builds N cells. Cell i has id i, 1+i%31 ASCII x characters, and values i+j for j in [0,1+i%7). Its next edge points to the previous even cell. Keep even cells; drop the all-cells array. Sum each kept id, text length, next id if present, and all values, modulo U. |

All particles use exact binary fractions. Quantization has no floating point boundary ambiguity.
Quicksort keeps equal values and uses the same partition swaps in both subjects.
The collect graph retains the even chain. Odd cells become unreachable before the checksum walk.
The native Luau constructors return plain tables with the same record and tree fields.
The workloads use no method dispatch or runtime library calls.
Both languages use the same arithmetic and hoist loop bounds that contain arithmetic or length reads.
Luau uses numeric loops and adds one at zero-based array accesses.

## Verification and cost

`cargo test -p ulua_bench` checks statistics, mismatch suppression, warm-up rejection, record suppression, and all accepted sources.
The source test reuses `crates/ulua/tests/support/tsc.rs` for stock TypeScript with the Ulua prelude.
It also compiles every source through the product C ABI at default compile limits.
The Luau source test compiles and loads each module without a workload call.
The measurement test supplies a fake workload and clock; it tests both warm-up conditions without real-time assertions.
The publication test uses a temporary Git index. It makes no commit.
Test headers state the measured costs. No test runs a benchmark workload.

## Captured results

A record with `worktree_dirty: true` predates the clean-crates policy. Re-run from committed workload code before use.

<!-- results:start -->

Session: 2026-10-19T05:23:56Z. Host ID: `host-7a4047f4a0ca82c1` (FNV-1a 64-bit hostname hash).

Machine: Darwin 27.0.0 arm64; MacBook Air. CPU: Apple M2; 8 (4 Performance and 4 Efficiency) cores. Power: Now drawing from 'AC Power'.

Luau: `c54f558b4d5748ab0658610b8ce0c432053e41eb`. Ulua: `f66c2e2121bc4623ce6bff678a1e8fdee76d8815`; worktree dirty: false.

Rust: rustc 1.95.0 (59807616e 2026-04-14). Profile: Release. Luau: O1, no interrupt callback.

Ulua limits: max_steps = 10000000000; max_memory_bytes = 1073741824; max_open_requests = 256.

| Workload | Checksum | luau ms (baseline) | ulua ms | ulua / luau |
|---|---:|---:|---:|---:|
| fib-recursive | 1346269 | 60.968 (1.00×) | 106.785 | 1.751× |
| fib-loop | 3449993952 | 111.935 (1.00×) | 243.719 | 2.177× |
| mandelbrot | 11189274 | 496.688 (1.00×) | 561.567 | 1.131× |
| primes | 17984 | 84.437 (1.00×) | 101.806 | 1.206× |
| sort | 3135319498 | 61.058 (1.00×) | 155.612 | 2.549× |
| tree | 393204 | 31.891 (1.00×) | 68.800 | 2.157× |
| queen | 2680 | 433.860 (1.00×) | 916.141 | 2.112× |
| particles | 77504288 | 81.668 (1.00×) | 169.212 | 2.072× |
| callbacks (Ulua methods; Luau loops) | 3403969141 | 100.187 (1.00×) | 137.832 | 1.376× |
| collect | 1011185172 | 127.764 (1.00×) | 255.048 | 1.996× |

Warm-up and noise (IQR / median; a withheld cell has no published timing):

- fib-recursive / luau: warm-up 245.680 ms, 4 iterations; IQR 0.176 ms (0.29%); ok.
- fib-recursive / ulua: warm-up 320.502 ms, 3 iterations; IQR 0.214 ms (0.20%); ok.
- fib-loop / luau: warm-up 340.708 ms, 3 iterations; IQR 0.988 ms (0.88%); ok.
- fib-loop / ulua: warm-up 728.391 ms, 3 iterations; IQR 0.530 ms (0.22%); ok.
- mandelbrot / luau: warm-up 1487.501 ms, 3 iterations; IQR 1.744 ms (0.35%); ok.
- mandelbrot / ulua: warm-up 1686.514 ms, 3 iterations; IQR 1.419 ms (0.25%); ok.
- primes / luau: warm-up 254.110 ms, 3 iterations; IQR 0.202 ms (0.24%); ok.
- primes / ulua: warm-up 305.146 ms, 3 iterations; IQR 0.401 ms (0.39%); ok.
- sort / luau: warm-up 243.457 ms, 4 iterations; IQR 0.249 ms (0.41%); ok.
- sort / ulua: warm-up 468.861 ms, 3 iterations; IQR 0.275 ms (0.18%); ok.
- tree / luau: warm-up 221.634 ms, 7 iterations; IQR 0.529 ms (1.66%); ok.
- tree / ulua: warm-up 209.489 ms, 3 iterations; IQR 0.538 ms (0.78%); ok.
- queen / luau: warm-up 1304.155 ms, 3 iterations; IQR 0.288 ms (0.07%); ok.
- queen / ulua: warm-up 2748.560 ms, 3 iterations; IQR 3.010 ms (0.33%); ok.
- particles / luau: warm-up 244.813 ms, 3 iterations; IQR 0.397 ms (0.49%); ok.
- particles / ulua: warm-up 507.564 ms, 3 iterations; IQR 0.943 ms (0.56%); ok.
- callbacks / luau: warm-up 304.920 ms, 3 iterations; IQR 0.358 ms (0.36%); ok.
- callbacks / ulua: warm-up 417.355 ms, 3 iterations; IQR 0.297 ms (0.22%); ok.
- collect / luau: warm-up 382.742 ms, 3 iterations; IQR 3.247 ms (2.54%); ok.
- collect / ulua: warm-up 762.171 ms, 3 iterations; IQR 3.958 ms (1.55%); ok.

<!-- results:end -->

## Boundary price

Set `LUAU_SOURCE_DIR` to the pinned checkout. Run from the repository root:

```sh
CARGO_NET_OFFLINE=true cargo run --release -p ulua_bench --bin boundary -- --stdout
CARGO_NET_OFFLINE=true cargo run --release -p ulua_bench --bin boundary -- --only host-into-script
CARGO_NET_OFFLINE=true cargo run --release -p ulua_bench --bin boundary -- --only script-into-host
```

The default command, without arguments, records both boundaries after a clean-crates check.
It writes `boundary.json` and the marked section below. It uses the same publication policy as `bench`.
`--stdout` and `--only` write no files. Both print the table and all observations.
A default run checks staged, unstaged, and untracked crate files before measurement and publication.
Commit the measured crate code before that run.

Each variant runs in a fresh child process, in sequence. Each region contains exactly 1,000 boundary calls.
The host region calls `f(a, b)` with `(i, 1)` for `i` from zero through 999.
The script region calls one export. Its loop calls the granted `hostAdd(i, 1)` 1,000 times.
Both regions return the checksum 500500. Each warm-up and sample must match this independent expected value.
The Luau floor uses the same addition function and while loop on the pinned VM.
The product uses the public C ABI and an explicit capability grant.
No measurement replaces a product object or changes product source.

The timer surrounds the region only. Compilation, setup, clock probes, output, and final checksum validation stay outside.
Stack operations, per-call result reads, and sum accumulation stay inside both region timers.
The policy requires 11 samples, a median, the 15% IQR gate, and 200 ms plus three warm-up iterations.
Each child measures 100,000 consecutive `Instant` pairs before setup.
It records the smallest positive difference as its quantum.
If no positive difference exists, or quantum exceeds 1% of the median region span, the report withholds that variant.
The report also withholds layers that depend on an invalid endpoint.

| Variant | Added layer |
|---|---|
| floor | Direct `lua_pcall`; script calls a plain `lua_CFunction`. No interrupt callback. |
| accounting | The product charging allocator and a matching step interrupt callback. |
| protected | A `lua_cpcall` wrapper, export table lookup, and the inner `lua_pcall`, as in the shim. |
| conversion | Numeric value tags, conversion, and result checks. The script variant also calls the typed Rust callback directly. |
| trampoline | Script boundary only: fixed argument array, revocation check, step charge, and descriptor dispatch to the Rust host callback. |
| shim | The real native shim, with the same hand-written Luau and a direct host callback. |
| ulua | The complete public ABI, compiled product source, and granted host callback. |

Layers use signed differences between adjacent medians. Shares divide each difference by the complete product median.
The floor share plus the layer shares totals 100% when all variants pass.
Negative differences retain noise and interactions. The IQR gate does not establish significance for differences.
The native intermediate variants are probes of the named layers. They are not product configurations.
The script conversion probe crosses the typed host ABI directly, so its price includes that direct call.
The trampoline probe then adds descriptor dispatch, revocation, and the host-call step charge.
The `protected` difference includes export lookup. It does not isolate protection from lookup.
The accounting difference combines the interrupt callback and allocator. These allocation-free regions do not price allocation-heavy workloads.
The native shim retains its full checks. Its delta includes work absent from the smaller probes.

`native/shim/ulua_shim.cpp` names `call`, `host_call`, `begin_run`, and the protected shim entry.
`crates/ulua/src/abi.rs` names `call_entry`, argument validation, `CString` export names, and the context guard.
`crates/ulua/src/capabilities.rs` adds callback panic protection and result validation.
The final residual combines the Rust ABI with product initialization and generated helper paths, including the post-call drain.
The shim probe uses hand-written Luau. Thus, the residual does not isolate Rust alone.
The report selects the largest positive non-floor layer as a shrink candidate and states its measured share.
A withheld layer prevents candidate selection. If no layer has a positive price, the report states that the call is the floor.

The quantum unit test uses fixed durations. It tests zeros, missing positives, the exact 1% boundary, fractional medians, and invalid spans.
Its header states its measured cost. No test runs the 100,000-pair probe or a timed boundary workload.

<!-- boundary:start -->

Session: 2026-10-18T02:06:22Z. Host ID: `host-7a4047f4a0ca82c1`.

Machine: Darwin 27.0.0 arm64; MacBook Air. CPU: Apple M2; 8 (4 Performance and 4 Efficiency) cores. Power: Now drawing from 'AC Power'.

Luau: `c54f558b4d5748ab0658610b8ce0c432053e41eb`. Ulua: `e233a7c8f73b2c55435a52fe65d5edb80d9217c0`; worktree dirty: false.

Rust: rustc 1.95.0 (59807616e 2026-04-14). Release; O1; max_steps = 10000000000; max_memory_bytes = 1073741824; max_open_requests = 256.

Each region has 1,000 boundary calls. Clock: std::time::Instant. Quantum: minimum positive span of 100,000 consecutive clock pairs.

| Boundary | Variant | ns/call | Quantum ns | IQR/median | Status |
|---|---|---:|---:|---:|---|
| host-into-script | floor | 22.708 | 41 | 0.18% | ok |
| host-into-script | accounting | 24.500 | 41 | 0.34% | ok |
| host-into-script | protected | 53.459 | 41 | 9.08% | ok |
| host-into-script | conversion | 54.125 | 41 | 11.47% | ok |
| host-into-script | shim | 62.458 | 41 | 5.97% | ok |
| host-into-script | ulua | 73.167 | 41 | 5.89% | ok |
| script-into-host | floor | 26.584 | 41 | 1.25% | ok |
| script-into-host | accounting | 21.250 | 41 | 0.69% | ok |
| script-into-host | protected | 21.417 | 41 | 0.59% | ok |
| script-into-host | conversion | 22.459 | 41 | 1.30% | ok |
| script-into-host | trampoline | 31.125 | 41 | 0.93% | ok |
| script-into-host | shim | 31.417 | 41 | 10.54% | ok |
| script-into-host | ulua | 32.250 | 41 | 0.26% | ok |

Layer prices use signed adjacent differences. Shares use the full product median as denominator. Negative prices retain process noise and interactions.

| Boundary | Layer | ns/call | Product share |
|---|---|---:|---:|
| host-into-script | VM floor | 22.708 | 31.04% |
| host-into-script | step interrupts and charging allocator | 1.792 | 2.45% |
| host-into-script | protected wrapper | 28.959 | 39.58% |
| host-into-script | numeric ABI conversion and direct typed callback (script only) | 0.666 | 0.91% |
| host-into-script | remaining native shim | 8.333 | 11.39% |
| host-into-script | rest: Rust ABI, guards and product setup/helpers | 10.709 | 14.64% |
| script-into-host | VM floor | 26.584 | 82.43% |
| script-into-host | step interrupts and charging allocator | -5.334 | -16.54% |
| script-into-host | protected wrapper | 0.167 | 0.52% |
| script-into-host | numeric ABI conversion and direct typed callback (script only) | 1.042 | 3.23% |
| script-into-host | host trampoline, callback dispatch and call charge | 8.666 | 26.87% |
| script-into-host | remaining native shim | 0.292 | 0.91% |
| script-into-host | rest: Rust ABI, guards and product setup/helpers | 0.833 | 2.58% |

host-into-script: shrink candidate: protected wrapper, 39.58% of the product call. This is a probe estimate.
- host-into-script / floor: warm-up 200.020 ms, 8802 iterations; observed median 22.708 ns/call; ok.
- host-into-script / accounting: warm-up 200.014 ms, 8144 iterations; observed median 24.500 ns/call; ok.
- host-into-script / protected: warm-up 200.009 ms, 3760 iterations; observed median 53.459 ns/call; ok.
- host-into-script / conversion: warm-up 200.036 ms, 3739 iterations; observed median 54.125 ns/call; ok.
- host-into-script / shim: warm-up 200.007 ms, 3297 iterations; observed median 62.458 ns/call; ok.
- host-into-script / ulua: warm-up 200.023 ms, 2811 iterations; observed median 73.167 ns/call; ok.

script-into-host: shrink candidate: host trampoline, callback dispatch and call charge, 26.87% of the product call. This is a probe estimate.
- script-into-host / floor: warm-up 200.006 ms, 8596 iterations; observed median 26.584 ns/call; ok.
- script-into-host / accounting: warm-up 200.005 ms, 9341 iterations; observed median 21.250 ns/call; ok.
- script-into-host / protected: warm-up 200.007 ms, 9251 iterations; observed median 21.417 ns/call; ok.
- script-into-host / conversion: warm-up 200.019 ms, 8702 iterations; observed median 22.459 ns/call; ok.
- script-into-host / trampoline: warm-up 200.017 ms, 6406 iterations; observed median 31.125 ns/call; ok.
- script-into-host / shim: warm-up 200.010 ms, 6343 iterations; observed median 31.417 ns/call; ok.
- script-into-host / ulua: warm-up 200.013 ms, 6102 iterations; observed median 32.250 ns/call; ok.

<!-- boundary:end -->
