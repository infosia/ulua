# cli — the `ulua` developer command

Status: contracted (D35). Evidence: `specs/tracking/p8-cli.md`.

## 1. Purpose and boundary

Each example host and the gate test helper (`crates/ulua/tests/support/host.rs`)
hold the same knowledge: the C and C++ flags, the per-OS
system libraries, and the place of the static library. A host
developer without a command writes this knowledge again. The `ulua`
command holds it once. A script author also needs a way to check and
run a script without a host of their own.

The host owns its build (invariant 1). The command gives answers and
artifacts: a check result, the output of a script run, the link flags,
and, as a convenience, a small host built whole. It writes no build
system files and does not watch files.

The command uses the product C ABI for each compile and each run. It
calls the same functions as a host, with the same defaults.

## 2. Surface

One binary, `ulua`, in the crate `crates/ulua_cli` (package
`ulua_cli`). The library of the crate has the entry
`execute(args, stdout, stderr) -> i32` that `main` calls; the tests
call it in process. The subcommands are §2.1 to §2.4. `ulua help`,
`ulua --help`, and no arguments print the usage text to stdout and give
exit 0. Each other form is a usage error.

### 2.1 `ulua check <file.ts>`

Compiles the file with `ulua_compile`, no grants, and the default
compile limits. A clean compile prints one line to stderr,
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
   `/std:c11` or `/std:c++17 /permissive-`.
5. The include directories are the directory of each `--host` file and
   the include directory of §5.
6. The C compiler is `$CC` if set, else `cc`. The C++ compiler is
   `$CXX` if set, else `c++`. On MSVC, the command finds `cl.exe` as
   the gate test helper does.
7. The compiler output goes to stderr only when the compile or the
   link fails.
8. `--run` runs the result with each `<arg>` after `--`. It gives the
   exit code of the host and does not change its stdout or stderr.
   A host that a signal stops gives exit 1.

The flags and the system libraries of §2.3 and §2.4 have one
definition. The gate test helper and the command both read it.

## 3. Exit codes and streams

| Exit | Meaning |
|---|---|
| 0 | Success |
| 1 | The library rejects or stops the script: any status other than `ULUA_OK` from a compile, an instance, or a run. `build` without `--run`: the compile or the link fails |
| 2 | A usage error, an I/O error, or an environment error (§5) |

- `build --run` gives the exit code of the host (§2.4 rule 8).
- Each error is one clear message on stderr. No panic (core principle 5).
- The command works offline and headless. It starts no download.
- stdout carries only the script output (`run`), the requested answer
  (`link-flags`, the usage text), or the host output (`build --run`).
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

1. The first line is `error[<code>]: <message>`, with the code and the
   message of `ulua_diagnostic`.
2. The `-->` line, the source line, and the caret line follow when the
   line is not zero and the source has that line. Else the diagnostic
   has the first line and the rule line only. A run diagnostic has no
   source position (abi.md §3).
3. The width of the gutter is the number of digits of the line number.
4. The diagnostic column counts bytes (abi.md §3). The caret goes
   below the character that starts at that byte. Each tab before it
   in the source line is a tab in the caret line. Each other
   character before it is one space.
5. A line ends at a line feed, a carriage return, U+2028, or U+2029, as
   the compiler counts lines. The rendered source line holds no line
   ending.
6. The rule line names the cause of the code:
   - a code `C<n>` gives `= rule: C<n> <title>`, with the title of
     section C<n> of `collisions.md`;
   - each other code that the compiler or the runtime gives (`syntax`,
     `parse`, `unsupported`, `limit:<kind>`, `trap:<kind>`,
     `host:<kind>`, `async:pending`, `internal`) has a one-line text.
   A code without a text gives no rule line.
7. No color. The output is the same bytes on each run.

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

## 6. Differences from the owner's prior project

| Prior command | Here | Reason |
|---|---|---|
| `check`, `run`, `link-flags`, `build` | §2.1 to §2.4 | Same purpose |
| `emit` (the generated C) | Not present | Generated Luau stays internal (invariant 6) |
| `boundary` (the crossing plan of each foreign call) | Not present | A value crosses the ABI only as `ulua_value` (abi.md §4); no plan exists |
| `bind` (a mirror from a C header) | Not present | The grant list is the only record of a host function; the library writes the `.d.ts` (D20) |
| `run --watch` (reload) | Not present | Reload is outside the current phases (D31) |
| Imports from disk | Not present | A module is one file (surface.md) |
| A summary line `error: N error(s)` | Not present | A compile gives one diagnostic |
| A build of the library from `link-flags` | Exit 2 with the build command | The build needs `LUAU_SOURCE_DIR` and takes minutes |
| Caret by character count | Caret by the character at the byte column | The diagnostic column counts bytes |

## 7. Exit criteria (pre-registered)

1. `ulua check` on each accept entry gives exit 0, the clean line on
   stderr, and empty stdout. On each reject entry it gives exit 1 and
   stderr that starts with `error[<code>]: <message>`, where the code
   and the message are the `expected-error` header of the entry.
2. `ulua run` on each script of `examples/typescript/` and
   `examples/run/` gives the stdout of its expected file without the
   last line of the runner (`error ...` or `stopped ...`). The exit
   code is 0 for an expected file without such a line, else 1.
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
   and a code without a rule text. A test reads the `C<n>` headings of
   `collisions.md` and compares them with the rule titles. A test
   gives each code of the reject corpus and of `corpus.md` §6 to the
   renderer and finds a rule line.
7. `sh tools/gate.sh full` and `sh tools/hygiene.sh` pass. No golden
   changes.
