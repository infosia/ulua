# Ulua

An **embedded scripting language for native applications**: a
**TypeScript-subset syntax on the Luau virtual machine**. A Rust
compiler checks the script and emits typed Luau source. The pinned
Luau compiler and VM run it inside your process, behind a C ABI with
the prefix `ulua_`.

Because the syntax is a subset of TypeScript, standard TypeScript
editor tooling works against it. Because the values are plain Luau
values (a number is a Luau number, an array is a Luau table), a script
needs no runtime layer that imitates JavaScript. Performance numbers
come with the benchmarks of P6.

Ulua is a language project, not a JavaScript runtime and not a
JavaScript binding.

## Why it exists

A host that runs user-authored logic usually picks one of two:

- **An embedded dynamic language** (Lua, JavaScript): fast iteration,
  but no static types, and the script can do anything the binding
  layer lets it do.
- **A general-purpose language runtime** (a JavaScript engine): broad
  compatibility, but a large runtime, its own event loop, and
  semantics that the host does not control.

Ulua takes a third position: **TypeScript syntax and tooling**, a
**sound subset** that the compiler checks before anything runs, and a
**small, sandboxed VM** that the host drives through a narrow C ABI.
Where the subset accepts a program, it prints what `node` prints;
where the two cannot agree, the compiler rejects the form or the
contract names the difference.

## Who it's for

Developers who embed scripts in a native host that **owns its main
loop** and **does not trust the scripts it runs**: mods in a game,
user rules in a server, plug-in logic in a tool. They want:

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

### Who it's *not* for

These limits are permanent, not gaps to be closed later:

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

## Tutorials

- [Ulua for TypeScript developers](docs/tutorial-typescript.md): what
  changes coming from TypeScript, with the complete list of rejected
  forms.
- [Ulua for C and C++ developers](docs/tutorial-c-cpp.md): the
  language from the host's side, and a step-by-step embedding
  walkthrough with the programs in [`examples/`](examples/).

Every command and every output in the tutorials was run against the
repository as committed.

## What you get

### A sound TypeScript subset

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

### A narrow C ABI

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

### Witnesses on every gate run

The corpus is the definition of the language. Each gate run checks
every entry against its golden output, and against four witnesses:
`tsc` (acceptance), `node` (behaviour), `luau-analyze` in strict mode
(the generated Luau), and the Luau VM through the C ABI.

## Build

The build needs Rust (the version in `rust-toolchain.toml`), a C and
C++ compiler, Node 24, and a Luau checkout at the revision in
`luau-revision.txt`:

```sh
npm install                                   # once: the pinned TypeScript
export LUAU_SOURCE_DIR=/path/to/luau          # checkout at luau-revision.txt
cargo build --release -p ulua                 # libulua.a and the dynamic library
```

The header is [`include/ulua.h`](include/ulua.h).

## Gate

```sh
sh tools/gate.sh quick    # corpus headers, tsc, node
sh tools/gate.sh full     # also fmt, clippy, tests, luau-analyze, the VM
sh tools/hygiene.sh
```

## Status

P0 to P4 are complete: the language core, arrays, strings, nullable
values, records, classes, exceptions, host interop with capabilities,
and async with host requests. P5 (hardening: sanitizers, fuzzing,
Linux and Windows gates) and P6 (performance) are next. The ABI has
no compatibility promise yet.

Design and phase records live in [`specs/`](specs/): `specs/plan.md`
holds the plan and the decisions, `specs/blocks/` the area contracts,
and `specs/tracking/` the evidence of each phase.

## License

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
