#!/bin/sh
# Run each collision probe under node and under the Luau CLI.
# Usage: LUAU=<path to the luau CLI> sh tools/probes.sh [name...]
# Without names, it runs every probe except number-format, which prints
# 80,019 lines; run that probe as specs/tracking/p05-measurements.md states.
set -eu
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
NODE=${NODE:-node}
LUAU=${LUAU:?set LUAU to the luau CLI built from LUAU_SOURCE_DIR}
cd "$repo_root/tools/probes"
if [ "$#" -eq 0 ]; then
    set -- $(ls *.mjs | sed 's/\.mjs$//' | grep -v '^number-format$')
fi
for name in "$@"; do
    printf '## %s\n### node\n```text\n' "$name"
    "$NODE" "$name.mjs"
    printf '```\n### luau\n```text\n'
    "$LUAU" "$name.luau"
    printf '```\n\n'
done
