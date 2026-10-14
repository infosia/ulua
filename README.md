# Ulua

Ulua runs **scripts that you do not trust** inside a native
application. Scripts are written in a subset of TypeScript. A Rust
compiler checks each script and emits typed Luau source, and the
pinned Luau VM runs it in a sandbox. Your application drives
everything through one C header with the prefix `ulua_`.

## The security model

Every script input is treated as hostile. Ulua gives the host these
guarantees:

- **A script reaches the host only through grants.** A script has no
  file system, no network, no clock, and no global environment. It
  can call a host function only if the host passed that function to
  `ulua_compile`. The host can revoke a grant at any time; the next
  call stops the run, and the host function is not called.
- **Compilation stays inside limits.** The host bounds the source size
  and the nesting depth. The compiler adds fixed bounds of its own, so
  no accepted script can push the generated Luau past a Luau limit.
  The compiler's work and output grow linearly with the source, and a
  gate test measures that growth for each construct.
- **Each run stays inside limits.** The host bounds the steps of each
  run, the memory of each instance, and the open async requests. A
  script that loops forever stops with `limit:steps`; a script that
  allocates without end stops with `limit:memory`.
- **A stop is final for the run, not for the instance.** A trap, a
  limit, or a host error ends the current run. No script `catch` and
  no `finally` runs after it, so a script cannot hide a stop or keep
  working past it. The instance keeps its state, and the next call on
  it is valid.
- **The script decides nothing about scheduling.** No thread and no
  event loop of Ulua runs script code. Script code runs only inside a
  call that the host makes: `ulua_instance_new`, `ulua_call`,
  `ulua_call_start`, and each settle of `ulua_pump`.
- **Only source crosses the boundary.** The C ABI accepts TypeScript
  source only. Generated Luau and bytecode stay internal, so no host
  API hands raw bytecode to the VM.
- **Nothing unwinds into the host.** Every function returns a status.
  No Rust panic, C++ exception, or Luau error crosses the ABI.

The host is trusted: a guarantee above can depend on a precondition
that the host keeps, and [`specs/blocks/abi.md`](specs/blocks/abi.md)
states each one.

## The language

The syntax is a subset of TypeScript, so a script author gets
completion, rename, and inline errors from any TypeScript editor.
Every accepted program type-checks under stock `tsc` with the Ulua
prelude and the declarations of the host's grants, which the library
writes as a `.d.ts` file.

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

Values are plain Luau values: a `number` is a Luau number, a `string`
is a UTF-8 Luau string, an array is a Luau table. The subset holds
numbers, strings, booleans, `T | null`, arrays, records, nominal
classes, closures, the `Error` family with `try`/`catch`/`finally`,
and `async`/`await` with `Promise<T>`.

Where the subset accepts a program, it prints what `node` prints. A
form that cannot agree with `node` is rejected at its source position
with a code that names the rule, or the language names the difference
and its measured outputs (for example, string lengths count UTF-8
bytes). There is no `undefined`, no `any`, no `eval`, no npm, and no
module system.

## Tutorials

- [Ulua for TypeScript developers](docs/tutorial-typescript.md): what
  a script can do, and what changes coming from TypeScript.
- [Ulua for C and C++ developers](docs/tutorial-c-cpp.md): how a host
  compiles, runs, limits, and grants, step by step, with the programs
  in [`examples/`](examples/).

Every command and every output in the tutorials was run against the
repository as committed. A gate step runs each program of the
TypeScript tutorial again.

## How it is checked

The corpus in [`corpus/`](corpus/) is the definition of the language.
Each gate run compares every entry with its golden output and with
four witnesses: `tsc` (acceptance), `node` (behaviour), `luau-analyze`
in strict mode (the generated Luau), and the Luau VM through the C
ABI. Each phase ends with an adversarial review that probes the
limits and the stops with hostile inputs.

## Build

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
values, records, classes, exceptions, host functions with grants, and
async with host requests. P5 (hardening: sanitizers, fuzzing, Linux
and Windows gates) and P6 (performance) are next. The ABI has no
compatibility promise yet.

Design and phase records are in [`specs/`](specs/): `specs/plan.md`
holds the plan and the decisions, `specs/blocks/` the contracts, and
`specs/tracking/` the evidence of each phase.

## License

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
