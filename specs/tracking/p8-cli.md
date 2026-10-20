# P8 developer command

Status: P8 COMPLETE 2026-10-20 (Phase Review rounds 1 and 2; `emit`
review round 1). Contract: `blocks/cli.md` Rev 4.

## Exit checklist

`blocks/cli.md` §7.

- [x] 1. `check` over the corpus: 228 entries without a `grants` header
- [x] 2. `run` over `examples/typescript/` and `examples/run/`, with the stop code compared
- [x] 3. `run` on trap entries
- [x] 4. `build --run` on `examples/hello/` and `examples/run/`
- [x] 5. `link-flags` names an existing library; the flags come from `crates/ulua_toolchain`
- [x] 6. Unit tests of §2 to §5 (`crates/ulua_cli`: 18 tests)
- [x] 7. `gate.sh full` and `hygiene.sh` pass; no golden changes
- [x] 8. `emit` equals the Luau that `ulua_compile` compiles: 90 accept entries; 143 reject entries agree with `check`
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
| gate step `emit-comparison`, test body (90 entries) | 0.72 to 1.32 s |
| the same step: warm / after a compiler change / first build | 1 s / 4 s / 28 s |

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

`emit` review round 1 (MINOR 7):

- `emit` stopped after the Rust compiler, so a native-stage rejection
  gave exit 0 where `check` gives 1. Fix: §2.5 rule 3 compiles with
  `ulua_compile` first.
- `-o` wrote in place and could name the input: `ulua emit ok.ts -o
  ok.ts` replaced the source. Fix: §2.5 rule 2, a new file and a
  rename; the input as output is a usage error.
- The comparison workspace had no fmt, clippy, or `--locked` check,
  no Windows suffix, and an understated cost. Fix: `tools/gate.sh`.
- An assertion that could not fail; a misplaced `-o` gave a wrong
  reason.

Corpus: the `expected-error` text is a description, so §7.1 compares
the code only. The 9 entries with `grants` are outside §7.1, because
the command has no grants.

## Windows (3668f24, cli.md Rev 5 §5.1)

- On Windows, `std::fs::canonicalize` gives a verbatim path
  (`\\?\C:\...`). `cl.exe` rejects it. The same `host.c` compiled with a
  plain path (exit 0) and failed with the verbatim path (C1083, exit 2).
  So `build_hosts_and_link_flags` failed.
- 3668f24 adds `ulua_toolchain::plain_path`. `gate.sh full` on Windows
  passed on one machine. Not run: a real UNC share, a volume without a
  drive letter, and Linux.
- The review of 3668f24 found 5 MINOR items:
  - User text printed in plain form, not as given.
  - An unsupported verbatim prefix gave a message without the path.
  - `CC` was converted.
  - The Windows test expected value did not resolve short names.
  - Tracking was not updated.
  Fix: cli.md Rev 5 §5.1 and the commit that implements it.
- The review of that fix found 1 MAJOR item and 4 MINOR items:
  - MAJOR: the rule 3 test gave the same value as the given path and
    the resolved path, so it could not fail.
  - A `--host` failure named the derived parent directory.
  - Rule 4 had no test for `--lib` and the environment values.
  - Two tests derived the expected path in different ways.
  - This note did not state the Windows runs of the fix.
- The §5.1 fix changes behaviour only on Windows. Its Windows tests did
  not compile or run on the machine of the fix (macOS).
- Windows x86_64 (owner's run, MSVC, 15b9cd9): `gate.sh full` passes;
  `cargo-test` 123 s, with the Windows tests of §5.1.
- `ulua_fuzz` gives its root to `tsc` and `luau-analyze` in plain form
  (3668f24). The fuzz smoke run on Windows is not run. The gate runs
  `fuzz-smoke` only in `gate.sh sanitize`.

## Gaps

- `run` and `check` take no grants. A script that calls a host
  function needs its host.
- `tools/docs-runner.sh` holds its own copy of the host flags and
  system libraries. It does not read `crates/ulua_toolchain`.
- The Linux run of the command is open (D28). Windows: `gate.sh full`
  passed on one machine at 15b9cd9, with the §5.1 fix.
- Rule 1 of §5.1 for `--host` files and the output directory has no
  direct test. The evidence is the `cl.exe` build of
  `build_hosts_and_link_flags` with verbatim arguments, on Windows.
- The CLI copies the mapping of the default compile limits from
  `crates/ulua/src/abi.rs`. The comparison of §7.8 finds a drift only
  where it changes accepted text; rule 3 compiles with `ulua_compile`
  first, so a rejection cannot drift.
