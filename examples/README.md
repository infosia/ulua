# Examples

Each directory holds a script and a C host that runs it. A gate test
builds every host against `include/ulua.h` and the static library,
runs it, and compares its standard output with `expected.txt`.

| Directory | Shows |
|---|---|
| [`hello/`](hello/) | Compile, instantiate, call an export, pass and return a string, the output function |
| [`capabilities/`](capabilities/) | Grant host functions, write their `.d.ts`, revoke a grant |
| [`async/`](async/) | An async host function: requests, completion, the pump, call tasks |
| [`limits/`](limits/) | Step and memory limits, a trap, and the instance after each stop |
| [`run/`](run/) | A general runner: compile a script with no grants and call its `main` |

Build the library, then one host (macOS):

```sh
export LUAU_SOURCE_DIR=/path/to/luau          # checkout at luau-revision.txt
cargo build --release -p ulua
mkdir -p target/examples
cc -std=c11 -Wall -Wextra -Wpedantic -Werror examples/hello/host.c -I include \
    target/release/libulua.a -o target/examples/hello \
    -lc++ -liconv -framework Security -framework CoreFoundation
target/examples/hello examples/hello/hello.ts
```

The comment at the top of each `host.c` holds the Linux link line.
The [C and C++ tutorial](../docs/tutorial-c-cpp.md) walks through the
hosts step by step.
