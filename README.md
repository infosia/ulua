# Ulua

Write scripts in TypeScript; run them on Luau. Ulua is an **embedded
scripting language for native applications**: a Rust compiler checks a
**TypeScript-subset** script and emits typed Luau source, and the
pinned **Luau** compiler and VM run it inside your process, behind a C
ABI with the prefix `ulua_`.

Running on Luau means running inside a battle-tested sandbox. Luau is
the language that Roblox develops and uses to run the code that its
creators publish, and its VM is built to run code that the host did
not write: each script gets an isolated environment with read-only
built-in libraries, and the host can interrupt it at any time
([Luau sandboxing](https://luau-lang.org/sandbox)). Ulua turns that
sandbox on for every instance (`luaL_sandbox`, `luaL_sandboxthread`),
and adds what the C ABI enforces on top: host-set limits, explicit
grants, and stops that no script can catch.

Because the syntax is a subset of TypeScript, standard TypeScript
editor tooling works against it. Because the values are plain Luau
values (a number is a Luau number, an array is a Luau table), a script
needs no runtime layer that imitates JavaScript. Ulua is a language
project, not a JavaScript runtime and not a JavaScript binding.

## Made for scripts you do not trust

Ulua fits a native host that **owns its main loop** and **runs code
that someone else wrote**: mods in a game, user rules in a server,
plug-in logic in a tool. It gives that host:

- **Hostile scripts stay inside their limits.** The host sets the
  source size, the nesting depth, the step budget, and the memory
  budget. A script that loops forever stops with `limit:steps`; a
  script that allocates without end stops with `limit:memory`. The
  instance stays usable after each stop, and no stop reaches a script
  `catch` or `finally`.
- **The host grants every capability.** A script reaches the host only
  through the host functions that the host passes to `ulua_compile`.
  The library writes the `.d.ts` of those functions, so the editor
  knows them. A grant can be revoked at any time.
- **Editor tooling with no plugin.** Every accepted program
  type-checks under stock `tsc` with the Ulua prelude and the host
  declarations. Completion, rename, and inline errors work in any
  TypeScript editor.
- **JavaScript behaviour by default.** A corpus of programs runs
  under Ulua and under `node` on every gate run. They agree byte for
  byte, except where `specs/blocks/collisions.md` names a measured
  divergence (for example: string lengths are UTF-8 byte counts).
- **Async that the host drives.** `async`, `await`, and `Promise`
  run in the job order of `node`. A host function can return a
  promise; the host completes the request when its work is done and
  pumps the instance. No event loop and no background thread runs
  script code.

## What Ulua leaves out

By design, and for good:

- **No npm and no existing TypeScript.** The subset rejects each form
  that neither agrees with `node` nor has a named divergence. Existing
  packages do not carry over.
- **No JavaScript value emulation.** A number is a Luau number, a
  string is a Luau string, an array is a Luau table. There is no
  `undefined`, no `any`, no prototype, no `eval`.
- **Not a standalone program runtime.** The host owns the loop and
  calls the functions that a script exports. A script has no file
  system, no network, and no clock unless the host grants a function
  for it.
- **No raw Luau.** The C ABI accepts TypeScript source only. Generated
  Luau and bytecode stay internal.

## A first script

```ts
type Point = { x: number; y: number };

class Counter {
  count: number = 0;
  add(n: number): number {
    this.count += n;
    return this.count;
  }
}

export function main(): void {
  const words: string[] = ["alpha", "beta", "gamma"];
  print(words.map((w: string): string => `<${w}>`).join(" "));
  const c = new Counter();
  c.add(2);
  print(`count ${c.add(3)}`);
}
```

`tsc` accepts a superset; Ulua narrows it. Numbers, strings, booleans,
`T | null`, arrays, records (structural object types), classes
(nominal), closures, the `Error` family with `try`/`catch`/`finally`,
and `async`/`await` with `Promise<T>` are in the subset. A form that
`tsc` accepts and Ulua cannot run as `node` runs it is rejected at its
source position, with a code that names the rule.

## Under the hood

### One C header

The host compiles a source string with a list of grants, makes an
instance with limits, and calls exports with typed values. Every
function returns a status; no panic and no C++ exception crosses the
boundary. The [C tutorial](docs/tutorial-c-cpp.md) builds a complete
host.

### Limits that the host sets

`ulua_compile_limits` bounds the source size and the nesting depth.
`ulua_run_limits` bounds the steps of each run, the memory of each
instance, and the open async requests. The compiler adds fixed bounds
that keep the generated Luau inside the Luau limits (for example, at
most 192 upvalues in one function). Each limit gives a diagnostic code;
none gives a crash.

### Four witnesses on every gate run

The corpus is the definition of the language. Each gate run checks
every entry against its golden output, and against four witnesses:
`tsc` (acceptance), `node` (behaviour), `luau-analyze` in strict mode
(the generated Luau), and the Luau VM through the C ABI.

## Performance

Ten workloads, each written once in Ulua and once in hand-written Luau
for the same pinned VM. Both versions of a workload give the same
integer checksum; the benchmark withholds a workload when the
checksums differ. The ratio is the Ulua time divided by the hand
Luau time: **lower is better**, hand Luau = 1.00×. Ulua runs through
the public C ABI with its limits on (step charges, the charging
allocator, index checks); hand Luau runs with no interrupt callback.
Each cell discards warm-up runs until 200 ms of measured execution
(at least three runs), then reports the median of 11 timed runs. A
cell with an interquartile range wider than 15% of the median is
withheld as noise. The workloads, the parameters, and every sample are
in [`crates/ulua_bench/`](crates/ulua_bench/README.md).

| Workload | hand Luau | Ulua | Ulua / hand |
|---|---:|---:|---:|
| mandelbrot | 496.9 ms | 571.4 ms | **1.15×** |
| primes | 85.8 ms | 101.7 ms | **1.19×** |
| fib-recursive | 61.4 ms | 107.3 ms | 1.75× |
| collect | 128.5 ms | 265.1 ms | 2.06× |
| fib-loop | 117.3 ms | 244.0 ms | 2.08× |
| tree | 31.5 ms | 73.6 ms | 2.33× |
| callbacks | 100.1 ms | 311.8 ms | 3.12× |
| queen | 433.5 ms | 1354.2 ms | 3.12× |
| particles | 83.0 ms | 289.9 ms | 3.49× |
| sort | 61.1 ms | 215.6 ms | 3.53× |

What the numbers show:

- **Numeric loops are near hand Luau.** mandelbrot is 1.15× and primes
  is 1.19×. The compiler emits counted loops as numeric `for`, and it
  emits the native Luau `%` where it can prove that both operands are
  non-negative integers. Other `%` sites use `math.fmod`, which agrees
  with JavaScript for a negative operand.
- **The cost is the safety that a hostile script needs.** sort (3.53×),
  particles (3.49×), and queen (3.12×) read and write arrays in their
  inner loops. Each access checks its index, because a read outside an
  array stops the run where JavaScript gives `undefined`. Each loop
  also has an interrupt point that charges steps, so the host can stop
  a script that does not end. In a measurement of the separate layers,
  the index check was 77% to 85% of the difference on sort and queen.
- **`callbacks` (3.12×) is the cost of the idiom.** The Ulua version
  uses `map`, `filter`, and `reduce` with a closure call per element;
  the hand version uses plain loops.
- **Calls and allocation cost less than array traffic.** fib-recursive
  (1.75×), tree (2.33×), and collect (2.06×) pay for the call frame
  and for the allocator that charges each allocation to the memory
  limit of the instance.

### Against roblox-ts

[roblox-ts](https://github.com/roblox-ts/roblox-ts) (v3.0.0 at
`68648b0c`) also compiles TypeScript to Luau. Its output of the same
ten workloads ran in the same harness, after mechanical source changes
(`null` to `undefined`, `.length` to `.size()`). This measurement used
an earlier Ulua (before the native `%`), so its Ulua column is slower
than the table above on primes, fib-loop, sort, and callbacks.

| Workload | roblox-ts / hand | Ulua / hand (earlier) |
|---|---:|---:|
| mandelbrot | 1.00× | 1.19× |
| fib-recursive | 1.01× | 1.75× |
| sort | 0.99× | 3.64× |
| primes | 1.15× | 3.10× |
| queen | 1.15× | 3.16× |
| particles | 1.16× | 3.46× |
| callbacks | 1.33× | 3.19× |
| collect | 1.36× | 2.10× |
| fib-loop | 1.71× | 2.26× |
| tree | 1.87× | 2.33× |

roblox-ts is faster on every row, and the reason is in its output: an
element access has no index check, a loop has no interrupt point and
no step or allocation charge, and `%` is the native Luau `%`, which
differs from JavaScript for a negative operand. That output fits code
that the developer writes and trusts. Ulua is for code that the host
does not trust, and it pays for each of these checks. Where roblox-ts
emits a slower form (a `while true` loop with a flag for a `for` loop
with a computed bound, a metatable constructor for a class), it is
slower than hand Luau too: fib-loop 1.71×, tree 1.87×.

### Boundary calls

One call across the C ABI, median of 11 regions of 1,000 calls:

| Direction | Luau floor | Ulua |
|---|---:|---:|
| host calls a script export | 24.6 ns | 110.3 ns |
| script calls a granted host function | 27.7 ns | 56.9 ns |

The floor is a direct `lua_pcall` or a plain `lua_CFunction` on the
same VM. The Ulua price adds the protected call, the export lookup,
argument checks, the revocation check of a grant, and the step charge
of a host call.

This is one benchmark set on one machine; treat the ratios as
indicative. The tables are from an Apple M2 (macOS, arm64), Release
build, Luau at `c54f558`. The workload record is from 2026-10-17 at
`535f2fe`; the boundary record is from 2026-10-16 at `a65b01d`; the
roblox-ts measurement is from 2026-10-17, before `535f2fe`.

## Learn more

- [Ulua for TypeScript developers](docs/tutorial-typescript.md): what
  changes coming from TypeScript, with the complete list of rejected
  forms.
- [Ulua for C and C++ developers](docs/tutorial-c-cpp.md): the
  language from the host's side, and a step-by-step embedding
  walkthrough with the programs in [`examples/`](examples/).

Every command and every output in the tutorials was run against the
repository as committed.

## Build and gate

The build needs Rust (the version in `rust-toolchain.toml`), a C and
C++ compiler, Node 24, and a Luau checkout at the revision in
`luau-revision.txt`:

```sh
npm install                                   # once: the pinned TypeScript
export LUAU_SOURCE_DIR=/path/to/luau          # checkout at luau-revision.txt
cargo build --release -p ulua                 # libulua.a and the dynamic library
```

The header is [`include/ulua.h`](include/ulua.h). The gate:

```sh
sh tools/gate.sh quick    # corpus headers, tsc, node
sh tools/gate.sh full     # also fmt, clippy, tests, luau-analyze, the VM, the docs
sh tools/hygiene.sh
```

## Status

P0 to P4 are complete: the language core, arrays, strings, nullable
values, records, classes, exceptions, host interop with capabilities,
and async with host requests. P5 (hardening) is complete:
sanitizers, fuzzing, fault injection, the compiler memory bound, a
run-time bound on the work of one step, and `gate.sh full` on macOS
arm64, Linux x86_64, and Windows (MSVC). P6 (performance) is
complete: Ulua runs at 1.15 to 3.53 times the time of hand-written
Luau on the same VM (see [Performance](#performance)). P7 (the
standard library) is complete: `Math`, `Number`, `find`, `Map`, `Set`,
string case, and `JSON`. The ABI has no compatibility
promise yet.

Design and phase records live in [`specs/`](specs/): `specs/plan.md`
holds the plan and the decisions, `specs/blocks/` the area contracts,
and `specs/tracking/` the evidence of each phase.

## License

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
