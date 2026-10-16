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

Session: 2026-10-16T02:12:51Z. Host ID: `host-7a4047f4a0ca82c1` (FNV-1a 64-bit hostname hash).

Machine: Darwin 27.0.0 arm64; MacBook Air. CPU: Apple M2; 8 (4 Performance and 4 Efficiency) cores. Power: Now drawing from 'AC Power'.

Luau: `c54f558b4d5748ab0658610b8ce0c432053e41eb`. Ulua: `40da51b137806161781649c62fb9095a154d2601`; worktree dirty: false.

Rust: rustc 1.95.0 (59807616e 2026-04-14). Profile: Release. Luau: O1, no interrupt callback.

Ulua limits: max_steps = 10000000000; max_memory_bytes = 1073741824; max_open_requests = 256.

| Workload | Checksum | luau ms (baseline) | ulua ms | ulua / luau |
|---|---:|---:|---:|---:|
| fib-recursive | 1346269 | 59.151 (1.00×) | 185.035 | 3.128× |
| fib-loop | 3449993952 | 112.445 (1.00×) | 863.433 | 7.679× |
| mandelbrot | 11189274 | 480.312 (1.00×) | 962.538 | 2.004× |
| primes | 17984 | 84.544 (1.00×) | 700.591 | 8.287× |
| sort | 3135319498 | 59.116 (1.00×) | 410.706 | 6.948× |
| tree | 393204 | 31.513 (1.00×) | 107.904 | 3.424× |
| queen | 2680 | 421.527 (1.00×) | 2036.000 | 4.830× |
| particles | 77504288 | 82.986 (1.00×) | 602.431 | 7.259× |
| callbacks (Ulua methods; Luau loops) | 3403969141 | 100.498 (1.00×) | 452.143 | 4.499× |
| collect | 1011185172 | 127.266 (1.00×) | 356.976 | 2.805× |

Warm-up and noise (IQR / median; a withheld cell has no published timing):

- fib-recursive / luau: warm-up 236.874 ms, 4 iterations; IQR 0.124 ms (0.21%); ok.
- fib-recursive / ulua: warm-up 553.238 ms, 3 iterations; IQR 2.433 ms (1.32%); ok.
- fib-loop / luau: warm-up 331.153 ms, 3 iterations; IQR 0.787 ms (0.70%); ok.
- fib-loop / ulua: warm-up 2583.306 ms, 3 iterations; IQR 0.264 ms (0.03%); ok.
- mandelbrot / luau: warm-up 1440.836 ms, 3 iterations; IQR 8.969 ms (1.87%); ok.
- mandelbrot / ulua: warm-up 2887.913 ms, 3 iterations; IQR 0.226 ms (0.02%); ok.
- primes / luau: warm-up 253.710 ms, 3 iterations; IQR 0.081 ms (0.10%); ok.
- primes / ulua: warm-up 2100.648 ms, 3 iterations; IQR 0.857 ms (0.12%); ok.
- sort / luau: warm-up 236.535 ms, 4 iterations; IQR 0.107 ms (0.18%); ok.
- sort / ulua: warm-up 1231.633 ms, 3 iterations; IQR 0.521 ms (0.13%); ok.
- tree / luau: warm-up 221.720 ms, 7 iterations; IQR 0.250 ms (0.79%); ok.
- tree / ulua: warm-up 326.534 ms, 3 iterations; IQR 1.101 ms (1.02%); ok.
- queen / luau: warm-up 1268.097 ms, 3 iterations; IQR 0.285 ms (0.07%); ok.
- queen / ulua: warm-up 6110.999 ms, 3 iterations; IQR 3.651 ms (0.18%); ok.
- particles / luau: warm-up 248.095 ms, 3 iterations; IQR 0.415 ms (0.50%); ok.
- particles / ulua: warm-up 1794.987 ms, 3 iterations; IQR 0.756 ms (0.13%); ok.
- callbacks / luau: warm-up 303.637 ms, 3 iterations; IQR 0.221 ms (0.22%); ok.
- callbacks / ulua: warm-up 1359.428 ms, 3 iterations; IQR 0.473 ms (0.10%); ok.
- collect / luau: warm-up 381.201 ms, 3 iterations; IQR 1.117 ms (0.88%); ok.
- collect / ulua: warm-up 1079.415 ms, 3 iterations; IQR 3.383 ms (0.95%); ok.

<!-- results:end -->
