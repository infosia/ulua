#!/bin/sh
# Standing gate. Usage: sh tools/gate.sh <quick|full>
#   quick: corpus witnesses (header, tsc, node).
#   full:  quick, then the hygiene scan.
#   Both run, when Cargo.toml exists: cargo fmt --check, clippy, test,
#   the Luau tools build, and the Luau witnesses. They need LUAU_SOURCE_DIR.
set -eu
case "${1:-}" in
    quick|full) shape=$1 ;;
    *) echo 'usage: sh tools/gate.sh <quick|full>' >&2; exit 2 ;;
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

step corpus-witnesses "$NODE" tools/witness/corpus.mjs
if [ -f Cargo.toml ]; then
    step cargo-fmt cargo fmt --all --check
    step cargo-clippy cargo clippy --quiet --workspace --all-targets -- -D warnings
    step cargo-test cargo test --quiet --workspace
    step luau-tools sh tools/luau-tools.sh
    step luau-witnesses "$NODE" tools/witness/luau.mjs
fi
if [ "$shape" = full ]; then
    step hygiene sh tools/hygiene.sh
fi
printf 'gate %s: PASS\n' "$shape"
