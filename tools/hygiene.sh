#!/bin/sh
# Repository hygiene scan. See tools/hygiene.mjs.
set -eu
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
exec "${NODE:-node}" "$repo_root/tools/hygiene.mjs"
