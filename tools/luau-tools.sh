#!/bin/sh
# Build the Luau CLI tools (luau, luau-analyze) from LUAU_SOURCE_DIR into
# target/luau-tools. The Luau witness uses luau-analyze.
# Usage: LUAU_SOURCE_DIR=<Luau checkout> sh tools/luau-tools.sh
# Cost: about one minute for a cold build; nothing when both tools exist.
set -eu
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
src=${LUAU_SOURCE_DIR:?set LUAU_SOURCE_DIR to the Luau checkout at luau-revision.txt}
want=$(cat "$repo_root/luau-revision.txt")
have=$(git -C "$src" rev-parse HEAD)
if [ "$have" != "$want" ]; then
    echo "LUAU_SOURCE_DIR is at $have; luau-revision.txt wants $want" >&2
    exit 1
fi
out="$repo_root/target/luau-tools"
if [ -x "$out/luau-analyze" ] && [ -x "$out/luau" ] && [ "$(cat "$out/revision" 2>/dev/null)" = "$want" ]; then
    exit 0
fi
mkdir -p "$out/build"
cmake -S "$src" -B "$out/build" -G "Unix Makefiles" -DCMAKE_BUILD_TYPE=Release \
    -DLUAU_BUILD_TESTS=OFF >/dev/null
cmake --build "$out/build" --target Luau.Repl.CLI Luau.Analyze.CLI -j 8 >/dev/null
cp "$out/build/luau" "$out/build/luau-analyze" "$out/"
echo "$want" > "$out/revision"
