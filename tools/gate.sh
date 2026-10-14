#!/bin/sh
# Standing gate. Usage: sh tools/gate.sh <quick|full|sanitize>
#   quick: corpus witnesses (header, tsc, node).
#   full:  quick, then the hygiene scan.
#   sanitize: the tests, the Luau witnesses, the examples, and the docs
#   witness on the ASan and UBSan build (pipeline.md §2.6), in
#   target/sanitize. Any sanitizer report fails it.
#   quick and full also run, when Cargo.toml exists: cargo fmt --check,
#   clippy, test, the Luau tools build, the Luau witnesses, and the docs
#   witness. sanitize skips fmt and clippy (the normal gate runs them).
#   Each shape needs LUAU_SOURCE_DIR. Cost on macOS arm64: full about
#   6 minutes, sanitize about 7 minutes (tracking/p5-hardening.md).
set -eu
case "${1:-}" in
    quick|full|sanitize) shape=$1 ;;
    *) echo 'usage: sh tools/gate.sh <quick|full|sanitize>' >&2; exit 2 ;;
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

if [ "$shape" = sanitize ]; then
    export ULUA_SANITIZE=1
    export CARGO_TARGET_DIR="$repo_root/target/sanitize"
    # LeakSanitizer is not available on every platform; leaks are a
    # P5.3 check of their own.
    # The gate's options come last, so a value from the environment
    # cannot turn halt_on_error off.
    export ASAN_OPTIONS="${ASAN_OPTIONS:+$ASAN_OPTIONS:}halt_on_error=1:abort_on_error=1:detect_leaks=0"
    export UBSAN_OPTIONS="${UBSAN_OPTIONS:+$UBSAN_OPTIONS:}halt_on_error=1:print_stacktrace=1"
    step cargo-test cargo test --quiet --workspace
    step luau-tools sh tools/luau-tools.sh
    step luau-witnesses "$NODE" tools/witness/luau.mjs
    step docs-runner sh tools/docs-runner.sh "$CARGO_TARGET_DIR/examples/docs-run"
    step docs-witness "$NODE" tools/witness/docs.mjs "$CARGO_TARGET_DIR/examples/docs-run" docs/tutorial-typescript.md
    printf 'gate sanitize: PASS\n'
    exit 0
fi
step corpus-witnesses "$NODE" tools/witness/corpus.mjs
if [ -f Cargo.toml ]; then
    step cargo-fmt cargo fmt --all --check
    step cargo-clippy cargo clippy --quiet --workspace --all-targets -- -D warnings
    step cargo-test cargo test --quiet --workspace
    step luau-tools sh tools/luau-tools.sh
    step luau-witnesses "$NODE" tools/witness/luau.mjs
    step docs-runner sh tools/docs-runner.sh "${CARGO_TARGET_DIR:-target}/examples/docs-run"
    step docs-witness "$NODE" tools/witness/docs.mjs "${CARGO_TARGET_DIR:-target}/examples/docs-run" docs/tutorial-typescript.md
fi
if [ "$shape" = full ]; then
    step hygiene sh tools/hygiene.sh
fi
printf 'gate %s: PASS\n' "$shape"
