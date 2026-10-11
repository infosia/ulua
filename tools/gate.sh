#!/bin/sh
# Standing gate. Usage: sh tools/gate.sh <quick|full>
#   quick: corpus witnesses (header, tsc, node).
#   full:  quick, then the hygiene scan.
# Later phases add cargo build, test, fmt, clippy, and the Luau witnesses.
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
if [ "$shape" = full ]; then
    step hygiene sh tools/hygiene.sh
fi
printf 'gate %s: PASS\n' "$shape"
