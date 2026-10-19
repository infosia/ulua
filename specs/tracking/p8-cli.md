# P8 developer command

Status: P8 open for `emit` (§2.5, owner request). §2.1 to §2.4 complete
2026-10-20 (Phase Review rounds 1 and 2). Contract: `blocks/cli.md` Rev 3.

## Exit checklist

`blocks/cli.md` §7.

- [x] 1. `check` over the corpus: 228 entries without a `grants` header
- [x] 2. `run` over `examples/typescript/` and `examples/run/`, with the stop code compared
- [x] 3. `run` on trap entries
- [x] 4. `build --run` on `examples/hello/` and `examples/run/`
- [x] 5. `link-flags` names an existing library; the flags come from `crates/ulua_toolchain`
- [x] 6. Unit tests of §2 to §5 (`crates/ulua_cli`: 18 tests)
- [x] 7. `gate.sh full` and `hygiene.sh` pass; no golden changes
- [ ] 8. `emit` equals the Luau that `ulua_compile` compiles
- [x] Phase Review

## Measurements

macOS arm64.

| Item | Value |
|---|---|
| Test bodies of `cargo test -p ulua_cli` | 2.44 s |
| `cargo test -p ulua_cli`, build included | 52.11 s |
| Host builds in the tests (5) | 1.54 s |
| `gate.sh full` at the round 2 tree | 304.22 s |
| `ulua check` on a 1,000,000-byte one-line file, stderr | 409 bytes |

## Failures before the fix

Review round 1 (MAJOR 1, MINOR 11):

- MAJOR: the command read the whole file and printed the whole source
  line. A 1,000,000-byte one-line file gave 1,065,711 bytes on stderr.
  Fix: §2.1 bounded read, §4 rule 6 window.
- The `-->` and rule lines had the wrong indent.
- Rule texts restated the code.
- A thrown message with line feeds forged diagnostic lines (§4 rule 7).
- The output userdata dropped before the instance.
- Other MINOR items: I/O errors without a path; `$CC` not split; a
  status without a code; usage errors without a reason; `build --run`
  held the host output; grants entries in the corpus loop; stop codes
  not compared.

Review round 2 (MINOR 6):

- Invalid UTF-8 before the column moved the caret.
- Format characters (U+202E) reached the terminal.
- Some file names were not escaped.
- `(NULL, 0)` from the output callback made a slice from null.
- Two contract texts were wrong: the 120-character count and the
  `execute` stream exception.

Corpus: the `expected-error` text is a description, so §7.1 compares
the code only. The 9 entries with `grants` are outside §7.1, because
the command has no grants.

## Gaps

- `run` and `check` take no grants. A script that calls a host
  function needs its host.
- `tools/docs-runner.sh` holds its own copy of the host flags and
  system libraries. It does not read `crates/ulua_toolchain`.
- The Linux and Windows runs of the command are open (D28).
