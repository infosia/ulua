# Ulua and Luau benchmark suite

This suite measures the product C ABI against hand-written Luau on the same pinned VM.
The suite is a published comparison. It is not a performance gate.
No product crate depends on this crate.

Source: [Luau](https://luau.org), [upstream repository](https://github.com/luau-lang/luau).
The captured record below states the revision from `luau-revision.txt`.
The `boundary` binary belongs to P6.2. This slice supplies the `bench` binary only.

Set `LUAU_SOURCE_DIR` to the pinned checkout. Run from the repository root:

```sh
CARGO_NET_OFFLINE=true cargo run --release -p ulua_bench --bin bench
CARGO_NET_OFFLINE=true cargo run --release -p ulua_bench --bin bench -- --only sort
```

Only a full run writes `results.json` and the marked table below.
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
The host ID is a stable hostname hash. The record omits personal machine names and paths.

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
The native Luau workloads use class tables for the same record and tree shapes.

## Verification and cost

`cargo test -p ulua_bench` checks statistics, mismatch suppression, warm-up rejection, record suppression, and all accepted sources.
The source test reuses `crates/ulua/tests/support/tsc.rs` for stock TypeScript with the Ulua prelude.
It also compiles every source through the product C ABI at default compile limits.
Test headers state the measured costs. No test runs a benchmark workload.

## Captured results

<!-- results:start -->

Session: 2026-10-16T01:38:03Z. Host ID: `host-9241a7361b723c69` (hostname hash).

Machine: Darwin 27.0.0 arm64; MacBook Air. CPU: Apple M2; 8 (4 Performance and 4 Efficiency) cores. Power: Now drawing from 'AC Power'.

Luau: `c54f558b4d5748ab0658610b8ce0c432053e41eb`. Ulua: `a14b8f18a5017693cd62793a6bb5d5979d5250c1`; worktree dirty: true.

Rust: rustc 1.95.0 (59807616e 2026-04-14). Profile: Release. Luau: O1, no interrupt callback.

Ulua limits: max_steps = 10000000000; max_memory_bytes = 1073741824; max_open_requests = 256.

| Workload | Checksum | luau ms (baseline) | ulua ms | ulua / luau |
|---|---:|---:|---:|---:|
| fib-recursive | 1346269 | 61.002 (1.00×) | 186.927 | 3.064× |
| fib-loop | 3449993952 | 113.835 (1.00×) | 1825.700 | 16.038× |
| mandelbrot | 11189274 | 504.002 (1.00×) | 981.937 | 1.948× |
| primes | 17984 | 80.791 (1.00×) | 704.371 | 8.718× |
| sort | 3135319498 | 52.251 (1.00×) | 406.805 | 7.786× |
| tree | 393204 | 35.687 (1.00×) | 108.716 | 3.046× |
| queen | 2680 | 326.974 (1.00×) | 2035.111 | 6.224× |
| particles | 77504288 | 84.088 (1.00×) | 611.665 | 7.274× |
| callbacks (Ulua methods; Luau loops) | 3403969141 | 102.249 (1.00×) | 458.246 | 4.482× |
| collect | 1011185172 | 127.720 (1.00×) | 571.052 | 4.471× |

Warm-up and noise (IQR / median; a withheld cell has no published timing):

- fib-recursive / luau: warm-up 242.386 ms, 4 iterations; IQR 0.276 ms (0.45%); ok.
- fib-recursive / ulua: warm-up 560.157 ms, 3 iterations; IQR 0.595 ms (0.32%); ok.
- fib-loop / luau: warm-up 342.056 ms, 3 iterations; IQR 0.648 ms (0.57%); ok.
- fib-loop / ulua: warm-up 5475.363 ms, 3 iterations; IQR 1.109 ms (0.06%); ok.
- mandelbrot / luau: warm-up 1511.122 ms, 3 iterations; IQR 0.984 ms (0.20%); ok.
- mandelbrot / ulua: warm-up 2946.093 ms, 3 iterations; IQR 1.488 ms (0.15%); ok.
- primes / luau: warm-up 242.232 ms, 3 iterations; IQR 0.230 ms (0.28%); ok.
- primes / ulua: warm-up 2113.943 ms, 3 iterations; IQR 0.678 ms (0.10%); ok.
- sort / luau: warm-up 209.373 ms, 4 iterations; IQR 0.073 ms (0.14%); ok.
- sort / ulua: warm-up 1220.753 ms, 3 iterations; IQR 1.798 ms (0.44%); ok.
- tree / luau: warm-up 214.122 ms, 6 iterations; IQR 0.587 ms (1.64%); ok.
- tree / ulua: warm-up 326.975 ms, 3 iterations; IQR 0.784 ms (0.72%); ok.
- queen / luau: warm-up 1007.652 ms, 3 iterations; IQR 1.472 ms (0.45%); ok.
- queen / ulua: warm-up 6103.054 ms, 3 iterations; IQR 2.432 ms (0.12%); ok.
- particles / luau: warm-up 254.493 ms, 3 iterations; IQR 0.621 ms (0.74%); ok.
- particles / ulua: warm-up 1835.372 ms, 3 iterations; IQR 3.240 ms (0.53%); ok.
- callbacks / luau: warm-up 314.171 ms, 3 iterations; IQR 0.512 ms (0.50%); ok.
- callbacks / ulua: warm-up 1396.475 ms, 3 iterations; IQR 1.314 ms (0.29%); ok.
- collect / luau: warm-up 386.038 ms, 3 iterations; IQR 2.158 ms (1.69%); ok.
- collect / ulua: warm-up 1720.556 ms, 3 iterations; IQR 2.611 ms (0.46%); ok.

<!-- results:end -->
