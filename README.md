# Ulua

An embedded scripting language with TypeScript-subset syntax. A Rust
compiler emits typed Luau source, and the Luau VM runs it inside a host
application through a C ABI.

**Status: P0 (founding).** The repository holds the language rules, the
seed corpus, and the witness gate. No compiler exists yet.

- Rules for agents: [CLAUDE.md](CLAUDE.md)
- Plan and decisions: [specs/plan.md](specs/plan.md)
- Accepted surface: [specs/blocks/surface.md](specs/blocks/surface.md)
- Collision rules: [specs/blocks/collisions.md](specs/blocks/collisions.md)
- Corpus format: [specs/blocks/corpus.md](specs/blocks/corpus.md)

## Gate

```sh
npm install          # once; installs the pinned TypeScript
sh tools/gate.sh quick
sh tools/gate.sh full
```

The gate needs Node 24. `TSC_JS` can name an existing `typescript.js`.

## Collision probes

```sh
LUAU=<luau CLI built from the pinned revision> sh tools/probes.sh
```

`luau-revision.txt` pins the Luau revision.

## License

Dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Unless you explicitly state otherwise, any contribution
intentionally submitted for inclusion in the work by you, as defined in
the Apache-2.0 license, shall be dual-licensed as above, without any
additional terms or conditions.
