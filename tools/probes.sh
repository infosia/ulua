#!/bin/sh
# Run each collision probe under node and under the Luau CLI.
# Usage: LUAU=<path to the luau CLI> sh tools/probes.sh [name...]
set -eu
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
NODE=${NODE:-node}
LUAU=${LUAU:?set LUAU to the luau CLI built from LUAU_SOURCE_DIR}
cd "$repo_root/tools/probes"
if [ "$#" -eq 0 ]; then
    set -- $(ls *.mjs | sed 's/\.mjs$//')
fi
for name in "$@"; do
    printf '## %s\n### node\n```text\n' "$name"
    "$NODE" "$name.mjs"
    printf '```\n### luau\n```text\n'
    "$LUAU" "$name.luau"
    printf '```\n\n'
done
