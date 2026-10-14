#!/bin/sh
# Build the Luau CLI tools (luau, luau-analyze) from LUAU_SOURCE_DIR into
# target/luau-tools. The Luau witness uses luau-analyze.
# Usage: LUAU_SOURCE_DIR=<Luau checkout> sh tools/luau-tools.sh
# Cost: about one minute for a cold build; nothing when both tools exist.
# On Windows, CMake uses its default generator (Visual Studio), and the
# tools have the suffix .exe. Under MSVC, the Luau CMakeLists names the
# target Luau.UnitTest, so LUAU_BUILD_TESTS is ON; only the two CLI
# targets build.
set -eu
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
src=${LUAU_SOURCE_DIR:?set LUAU_SOURCE_DIR to the Luau checkout at luau-revision.txt}
want=$(cat "$repo_root/luau-revision.txt")
have=$(git -C "$src" rev-parse HEAD)
if [ "$have" != "$want" ]; then
    echo "LUAU_SOURCE_DIR is at $have; luau-revision.txt wants $want" >&2
    exit 1
fi
case "$(uname -s)" in
    MINGW* | MSYS* | CYGWIN*) generator= exe=.exe tests=ON ;;
    *) generator='Unix Makefiles' exe= tests=OFF ;;
esac
out="$repo_root/target/luau-tools"
if [ -x "$out/luau-analyze$exe" ] && [ -x "$out/luau$exe" ] && [ "$(cat "$out/revision" 2>/dev/null)" = "$want" ]; then
    exit 0
fi
mkdir -p "$out/build"
cmake -S "$src" -B "$out/build" ${generator:+-G "$generator"} -DCMAKE_BUILD_TYPE=Release \
    -DLUAU_BUILD_TESTS=$tests >/dev/null
cmake --build "$out/build" --config Release --target Luau.Repl.CLI Luau.Analyze.CLI -j 8 >/dev/null
# A multi-config generator writes the tools to build/Release.
bin="$out/build"
[ -f "$bin/luau$exe" ] || bin="$out/build/Release"
cp "$bin/luau$exe" "$bin/luau-analyze$exe" "$out/"
echo "$want" > "$out/revision"
