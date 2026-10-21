# cli — the `ulua` developer command

Status: contracted (D35). Evidence: `specs/tracking/p8-cli.md`.

Rev 1: §2, §2.1, §2.4, §3, §4 after the P8 review round 1.
Rev 2: §2, §3, §4 rules 4, 6, 7 after the P8 review round 2.
Rev 3: §2.5 `emit` (owner request); §1, §2, §3, §6, §7.
Rev 4: §2.5 rules 2 and 3 after the emit review.
Rev 5: §5.1 plain paths on Windows, after the review of the Windows
change.
Rev 6: §2.4 rule 4 names `/TC` and `/TP`, as `crates/ulua_toolchain`
gives them.

## 1. Purpose and boundary

Each example host and the gate test helper (`crates/ulua/tests/support/host.rs`)
hold the same knowledge: the C and C++ flags, the per-OS
system libraries, and the place of the static library. A host
developer without a command writes this knowledge again. The `ulua`
command holds it once. A script author also needs a way to check and
run a script without a host of their own.

The host owns its build (invariant 1). The command gives answers and
artifacts: a check result, the output of a script run, the generated
Luau to read, the link flags,
and, as a convenience, a small host built whole. It writes no build
system files and does not watch files.

The command uses the product C ABI for each compile and each run. It
calls the same functions as a host, with the same defaults.

## 2. Surface

One binary, `ulua`, in the crate `crates/ulua_cli` (package
`ulua_cli`). The library of the crate has the entry
`execute(args, stdout, stderr) -> i32` that `main` calls; the tests
call it in process. `execute` writes each stream of the command to
these writers. The one exception is the host of `build --run`, which
inherits the streams of the process (§2.4 rule 8). The subcommands are §2.1 to §2.5. `ulua help`,
`ulua --help`, and no arguments print the usage text to stdout and give
exit 0. Each other form is a usage error. A usage error prints one line
`ulua: <reason>` that names the wrong or missing argument, then the
usage text, both to stderr.

### 2.1 `ulua check <file.ts>`

Compiles the file with `ulua_compile`, no grants, and the default
compile limits. The command reads at most `max_source_bytes` + 1 bytes
of the default compile limits. A longer file gets the compiler
diagnostic `limit:source-bytes`; the command never holds more of the
file. A clean compile prints one line to stderr,
`check: <file as given>: no errors`, and nothing to stdout. A rejected
file prints the diagnostic of §4 to stderr. No artifact.

### 2.2 `ulua run <file.ts>`

Compiles the file as §2.1 does. Then it makes an instance with the
default run limits and the output function, and runs the export
`main` as the runner of `examples/run/` does: `ulua_call_start`,
`ulua_pump`, `ulua_task_result`.

1. Each line that the script prints goes to stdout at once, with a
   line feed. stdout carries nothing else.
2. A module without the export `main` prints
   `run: <file as given>: no export main` to stderr and gives exit 0.
   A module has zero or more exports (D27).
3. An export `main` with one or more parameters is a usage error.
4. The result value of `main` is not printed.
5. A stop (a trap, a run limit, a host error, `async:pending`) prints
   the diagnostic of §4 to stderr. The lines that the script printed
   before the stop stay on stdout.
6. The script has no grants. A script that calls a host function is
   rejected with the compiler diagnostic (collisions.md C18).

### 2.3 `ulua link-flags [--cc unix|msvc]`

Prints to stdout, one item on each line, what a host build adds to
link the static library:

1. the include directory, as `-I<dir>` (`unix`) or `/I<dir>` (`msvc`);
2. the static library path;
3. each system library of the platform, in the order of the gate test
   helper. A macOS framework is one line: `-framework Security`.

`--cc` selects the spelling. The default is `msvc` on a Windows MSVC
build and `unix` on each other build. The paths come from §5.

### 2.4 `ulua build --host <file>... [-o <dir>] [--run [-- <arg>...]]`

Compiles and links the host sources with the static library into
`<dir>/<name>`.

1. `<name>` is the stem of the first `--host` file plus the platform
   executable suffix.
2. The default `<dir>` is `ulua-build` in the directory of the first
   `--host` file. The command writes no file to the working directory
   and no file outside `<dir>`.
3. A `.c` file compiles as C11. A `.cc`, `.cpp`, or `.cxx` file
   compiles as C++17. If one source is C++, the C++ driver links.
   Another extension is a usage error.
4. The flags are the flags of the gate test helper: on Unix,
   `-std=c11` or `-std=c++17`, `-Wall -Wextra -Wpedantic -Werror`; on
   MSVC, `/nologo /W4 /WX /MD /D_CRT_SECURE_NO_WARNINGS` with
   `/TC /std:c11` or `/TP /std:c++17 /permissive-`.
5. The include directories are the directory of each `--host` file and
   the include directory of §5.
6. The C compiler is `$CC` if set, else `cc`. The C++ compiler is
   `$CXX` if set, else `c++`. The value splits at ASCII white space:
   the first word is the program, and each other word is an argument
   before the flags of rule 4. On MSVC, the command finds `cl.exe` as
   the gate test helper does.
7. The compiler output goes to stderr only when the compile or the
   link fails.
8. `--run` runs the result with each `<arg>` after `--`. The host
   inherits the stdin, stdout, and stderr of the command, so its
   output appears while it runs. The command gives the exit code of
   the host. A host that a signal stops gives exit 1.

The flags and the system libraries of §2.3 and §2.4 have one
definition. The gate test helper and the command both read it.

### 2.5 `ulua emit <file.ts> [-o <file>]`

Prints the generated Luau of the file, for a developer to read.

1. The text is the Luau that `ulua_compile` gives to the Luau compiler
   for the same source, no grants, and the default compile limits,
   byte for byte. The command calls the compiler of `ulua_compiler`
   with the same limits, on a thread with the same stack size
   (`COMPILER_STACK_BYTES`), as the corpus runner does for
   `--emit-luau` (pipeline.md §8).
2. Without `-o`, the text goes to stdout. With `-o`, the command
   writes the text to a new file in the directory of the output, then
   renames it to the output, and prints
   `emit: <file as given>: <output as given>` to stderr. A failure
   removes the new file and leaves the output unchanged. An output
   that names the input file is a usage error.
3. The command first compiles the file with `ulua_compile`, as `check`
   does. A file that `ulua_compile` rejects, at any stage, gives the
   stderr and the exit code of `check`. stdout stays empty, and the
   command creates or changes no `-o` file. So `emit` and `check`
   agree on each file.
4. The text is an output only. The C ABI has no function that returns
   it, and no function or command accepts Luau (invariant 6). The
   text is not a stable interface: a compiler change can change any
   byte of it. It does not run alone: the product gives the chunk its
   arguments (the prelude and the grants, pipeline.md).
5. The text is not escaped: it is an artifact, as the source file is.

## 3. Exit codes and streams

| Exit | Meaning |
|---|---|
| 0 | Success |
| 1 | The library rejects or stops the script: any status other than `ULUA_OK` from a compile, an instance, or a run. `build` without `--run`: the compile or the link fails |
| 2 | A usage error, an I/O error, or an environment error (§5) |

- `build --run` gives the exit code of the host (§2.4 rule 8).
- Each error is one clear message on stderr. An I/O error names the
  path or the program and the OS error. No panic (core principle 5).
- A library status without a diagnostic code prints
  `error[abi:status]: <status name> (<value>)`, for example
  `error[abi:status]: ULUA_BUSY (5)`.
- The command works offline and headless. It starts no download.
- stdout carries only the script output (`run`), the requested answer
  (`link-flags`, `emit` without `-o`, the usage text), or the host
  output (`build --run`).
  Each other line goes to stderr.

## 4. Diagnostic rendering

`check`, `run`, and `build` print a compiler diagnostic in one shape.
`run` uses the same shape for a stop.

```text
error[C6]: <message>
 --> <file>:<line>:<column>
  |
3 | const value = values.at(0);
  |                      ^
  = rule: C6 Arrays
```

With a line number of two digits:

```text
error[C4]: <message>
  --> <file>:12:21
   |
12 |   const x: number = 2 ** 3;
   |                     ^
   = rule: C4 Numbers
```

1. The first line is `error[<code>]: <message>`, with the code and the
   message of `ulua_diagnostic`.
2. The `-->` line, the source line, and the caret line follow when the
   line is not zero and the source has that line. Else the diagnostic
   has the first line and the rule line only. A run diagnostic has no
   source position (abi.md §3).
3. The width `w` of the gutter is the number of digits of the line
   number. The `-->` line starts with `w` spaces. The `|` lines without
   a number and the rule line start with `w` + 1 spaces.
4. The diagnostic column counts bytes of the source as read
   (abi.md §3). The command finds the line and the caret on these
   bytes. Each byte that is not part of valid UTF-8 renders as one
   U+FFFD character at its own byte. The caret goes below the
   character that starts at the column byte. Each tab before it
   in the source line is a tab in the caret line. Each other
   character before it is one space.
5. A line ends at a line feed, a carriage return, U+2028, or U+2029, as
   the compiler counts lines. The rendered source line holds no line
   ending.
6. The rendered source line holds at most 120 source characters,
   before the escapes of rule 7. A longer
   line shows the 120 characters that start 60 characters before the
   caret character, or at the start of the line if fewer characters
   come before it. The window does not go past the end of the line:
   a window that reaches the end starts 120 characters before the
   end. Each cut side gets `...`. The caret line follows the window.
7. The message and the rendered source line come from the script.
   Each control character in them (U+0000 to U+001F, U+007F to
   U+009F) and each format character (Unicode general category Cf,
   for example U+202E and U+200B) prints as `\u{h}`: the code point
   in lowercase hex, with at least two digits. A tab in the source
   line is the one exception (rule 4). So a message prints on one line, and
   no script byte reaches the terminal as a control code or changes
   the order in which the line shows. Each file name that the
   command prints, on each line of §2 to §4, gets the same escape. The caret
   counts each `\u{hh}` as its printed characters.
8. The rule line names the cause of the code:
   - a code `C<n>` gives `= rule: C<n> <title>`, with the title of
     section C<n> of `collisions.md`;
   - a code `trap:<kind>` gives `= rule: <cause> (<rule>)`, with the
     rule of the kind in corpus.md §6, for example
     `= rule: an index outside the array (C6)`;
   - each other code that the compiler or the runtime gives (`syntax`,
     `parse`, `unsupported`, `limit:<kind>`, `host:<kind>`,
     `async:pending`, `internal`) has a one-line text that states
     the cause, not the code again. Two codes do not share a text.
   A code without a text gives no rule line.
9. No color. The output is the same bytes on each run.

## 5. Path of the static library and the include directory

`link-flags` and `build` take the paths in this order:

1. the options `--lib <file>` and `--include <dir>`;
2. the environment variables `ULUA_LIB` and `ULUA_INCLUDE`;
3. in this repository: `target/release/` with the platform library
   name (`libulua.a`, or `ulua.lib` on MSVC), and `include/`.

Each source applies to each path on its own: `--lib` and
`ULUA_INCLUDE` can be used together. The repository default applies
when the command is built from this repository and `include/ulua.h`
exists there. If the library file does not exist, the command gives
exit 2 with a message that names the three sources and the build
command `cargo build --release -p ulua`. The command does not start
that build: the build needs `LUAU_SOURCE_DIR` and takes minutes.

### 5.1 Plain paths on Windows

On Windows, the canonical form of a path has a verbatim prefix
(`\\?\`), and `cl.exe` rejects it (tracking/p8-cli.md).

1. The command gives a derived path in plain form. A derived path is
   the canonical form of the library, the include directory, a
   `--host` file, or the `build` output. Each `link-flags` line and
   each argument that `build` gives to a compiler is a derived path.
2. The plain form of `\\?\C:\<rest>` is `C:\<rest>`. The plain form of
   `\\?\UNC\<server>\<share>\<rest>` is `\\<server>\<share>\<rest>`.
   A path without a verbatim prefix does not change.
3. If a derived path has another verbatim prefix (for example
   `\\?\Volume{...}\`), the command gives exit 2. The message names
   the path as given and states that the path has no plain form.
4. Text that the user gives prints as given, with the escape of §4
   rule 7: each argument, each file name, and each value of `--lib`,
   `--include`, `ULUA_LIB`, `ULUA_INCLUDE`, `CC`, and `CXX`. The
   command does not convert it.
5. The words of `CC` and `CXX` go to the operating system unchanged
   (§2.4 rule 6).
6. `emit` compares the input and the output in canonical form (§2.5
   rule 2). It prints neither form.

On other platforms, the canonical form has no prefix, and §5.1 changes
nothing.

## 6. Differences from the owner's prior project

| Prior command | Here | Reason |
|---|---|---|
| `check`, `run`, `link-flags`, `build` | §2.1 to §2.4 | Same purpose |
| `emit` (the generated C) | §2.5, the generated Luau | An output for a developer only; no C ABI function returns it or accepts Luau (invariant 6), as for the corpus runner |
| `boundary` (the crossing plan of each foreign call) | Not present | A value crosses the ABI only as `ulua_value` (abi.md §4); no plan exists |
| `bind` (a mirror from a C header) | Not present | The grant list is the only record of a host function; the library writes the `.d.ts` (D20) |
| `run --watch` (reload) | Not present | Reload is outside the current phases (D31) |
| Imports from disk | Not present | A module is one file (surface.md) |
| A summary line `error: N error(s)` | Not present | A compile gives one diagnostic |
| A build of the library from `link-flags` | Exit 2 with the build command | The build needs `LUAU_SOURCE_DIR` and takes minutes |
| Caret by character count | Caret by the character at the byte column | The diagnostic column counts bytes |

## 7. Exit criteria (pre-registered)

1. This criterion covers each corpus entry without a `grants` header
   (corpus.md §7). The command has no grants (§2.2 rule 6), so an
   entry with grants is outside it.
   `ulua check` on each accept entry gives exit 0, the clean line on
   stderr, and empty stdout. On each reject entry it gives exit 1 and
   stderr that starts with `error[<code>]: `, where the code is the
   rule id of the `expected-error` header of the entry. The text of
   the header is a description; the corpus compares only the code
   (corpus.md §2).
2. `ulua run` on each script of `examples/typescript/` and
   `examples/run/` gives the stdout of its expected file without the
   last line of the runner (`error ...` or `stopped ...`). The exit
   code is 0 for an expected file without such a line, else 1. For
   such a line, the code in the first stderr line equals the code in
   that line (`error <code> at ...` or `stopped <code>: ...`).
3. `ulua run` on a trap entry gives the lines before the trap on
   stdout, the trap code on stderr, and exit 1. A trap entry without
   output before the trap gives empty stdout.
4. `ulua build --host examples/hello/host.c --run -- examples/hello/hello.ts`
   gives the stdout of `examples/hello/expected.txt` and exit 0. The
   same holds for `examples/run/host.c` with `examples/run/hello.ts`.
5. After `cargo build --release -p ulua`, `link-flags` names a library
   that exists. Its system libraries equal the libraries that the gate
   test helper links, from the one definition of §2.
6. Unit tests cover each subcommand's success path, each exit code of
   §3, the usage text, the order of §5 (an option before the
   environment before the repository), and each rule of §4: a snippet
   with its caret, a tab and a multi-byte character before the caret,
   a CRLF source, the line 0 form, a line past the end of the source,
   a code without a rule text, a two-digit gutter, a long line cut on
   each side, and a control character in a message and in a source
   line. A test reads the trap table of corpus.md §6 and finds the
   rule of each kind in its rule line. A test finds no two codes with
   one text. A test checks that a file larger than the source limit
   gives `limit:source-bytes` and stderr of a bounded size. A test
   reads the `C<n>` headings of `collisions.md` and compares them with
   the rule titles. A test
   gives each code of the reject corpus and of `corpus.md` §6 to the
   renderer and finds a rule line. On Windows, tests cover each rule
   of §5.1. An expected path in a test resolves short names and links
   as the command does.
7. `sh tools/gate.sh full` and `sh tools/hygiene.sh` pass. No golden
   changes.
8. `emit`: for each accept entry without a `grants` header, a test
   captures the Luau that `ulua_compile` gives to the Luau compiler
   (the `CompileEvent::Rust` observation of `compile_measurement`) and
   compares it with the `emit` output, byte for byte. On each reject
   entry, `emit` gives the same stderr and exit code as `check`, and
   empty stdout. With `-o`, a rejected file leaves an existing output
   file unchanged.
