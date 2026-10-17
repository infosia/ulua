#!/bin/sh
# Standing gate. Usage: sh tools/gate.sh <quick|full|sanitize|timing>
#   quick: corpus witnesses (header, tsc, node).
#   full:  quick, then the remainder shadow checks and the hygiene scan.
#   sanitize: the tests, the Luau witnesses, the examples, and the docs
#   witness on the ASan and UBSan build (pipeline.md §2.6), in
#   target/sanitize. Any sanitizer report fails it.
#   timing: the tests that assert time ratios (the `witnesses` test
#   binary, feature `timing`; pipeline.md §3.10). Run it at the close
#   of a slice and with each change to the checker or the emitter.
#   quick and full also run, when Cargo.toml exists: cargo fmt --check,
#   clippy, test, the Luau tools build, the Luau witnesses, and the docs
#   witness. sanitize skips fmt and clippy (the normal gate runs them).
#   Each shape needs LUAU_SOURCE_DIR. Measured cost on macOS arm64,
#   warm build: full about 2 to 3 minutes, timing about 6 minutes,
#   sanitize about 8 to 10 minutes (tracking/p5-hardening.md).
set -eu
case "${1:-}" in
    quick|full|sanitize|timing) shape=$1 ;;
    *) echo 'usage: sh tools/gate.sh <quick|full|sanitize|timing>' >&2; exit 2 ;;
esac
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"
NODE=${NODE:-node}

step() {
    name=$1
    shift
    start=$(date +%s)
    if "$@"; then
        printf 'PASS %s (%ss)\n' "$name" "$(( $(date +%s) - start ))"
    else
        printf 'FAIL %s\n' "$name"
        exit 1
    fi
}

if [ "$shape" = timing ]; then
    step luau-tools sh tools/luau-tools.sh
    step timing cargo test --quiet -p ulua --features timing --test witnesses
    printf 'gate timing: PASS\n'
    exit 0
fi
if [ "$shape" = sanitize ]; then
    export ULUA_SANITIZE=1
    export CARGO_TARGET_DIR="$repo_root/target/sanitize"
    # LeakSanitizer is not available on every platform; leaks are a
    # P5.3 check of their own.
    # The gate's options come last, so a value from the environment
    # cannot turn halt_on_error off.
    export ASAN_OPTIONS="${ASAN_OPTIONS:+$ASAN_OPTIONS:}halt_on_error=1:abort_on_error=1:detect_leaks=0"
    export UBSAN_OPTIONS="${UBSAN_OPTIONS:+$UBSAN_OPTIONS:}halt_on_error=1:print_stacktrace=1"
    step luau-tools sh tools/luau-tools.sh
    # ulua_bench links its own unsanitized Luau hosts (benchmarks.md
    # §2); it is a measurement tool, and gate full runs its tests.
    step cargo-test cargo test --quiet --workspace --exclude ulua_bench
    step luau-witnesses "$NODE" tools/witness/luau.mjs
    step docs-runner sh tools/docs-runner.sh "$CARGO_TARGET_DIR/examples/docs-run"
    step docs-witness "$NODE" tools/witness/docs.mjs "$CARGO_TARGET_DIR/examples/docs-run" docs/tutorial-typescript.md
    # pipeline.md §9.5: each fuzz target, a fixed seed and input count.
    step fuzz-smoke cargo run --quiet -p ulua_fuzz -- --smoke
    # pipeline.md §10: the allocation-failure sweep (feature fault-injection).
    step fault-sweep cargo test --quiet -p ulua --features fault-injection --test faults
    printf 'gate sanitize: PASS\n'
    exit 0
fi
step corpus-witnesses "$NODE" tools/witness/corpus.mjs
if [ -f Cargo.toml ]; then
    step cargo-fmt cargo fmt --all --check
    step cargo-clippy cargo clippy --quiet --workspace --all-targets --features ulua/timing,ulua/fault-injection -- -D warnings
    step luau-tools sh tools/luau-tools.sh
    step cargo-test cargo test --quiet --workspace
    step luau-witnesses "$NODE" tools/witness/luau.mjs
    step docs-runner sh tools/docs-runner.sh "${CARGO_TARGET_DIR:-target}/examples/docs-run"
    step docs-witness "$NODE" tools/witness/docs.mjs "${CARGO_TARGET_DIR:-target}/examples/docs-run" docs/tutorial-typescript.md
fi
if [ "$shape" = full ]; then
    # pipeline.md §5.25: each native % against math.fmod, over the corpus
    # and the benchmark workloads. Cost with the build: about 4 s and 9 s.
    step remainder-shadow-corpus env ULUA_REMAINDER_SHADOW=1 cargo test --quiet -p ulua --features remainder-shadow --test runtime p6_remainder::shadow_corpus_accept_and_trap -- --exact
    step remainder-shadow-workloads env ULUA_REMAINDER_SHADOW=1 cargo test --quiet -p ulua_bench --features remainder-shadow --bin bench tests::shadow_ten_workloads -- --exact
    step hygiene sh tools/hygiene.sh
fi
printf 'gate %s: PASS\n' "$shape"
