#!/bin/sh
# Builds examples/run/host.c against the debug static library that
# `cargo test` leaves, for the docs witness. Usage: sh tools/docs-runner.sh <out>
set -eu
out=$1
mkdir -p "$(dirname "$out")"
case "$(uname -s)" in
    Darwin) libs="-lc++ -liconv -framework Security -framework CoreFoundation" ;;
    *) libs="-lstdc++ -lpthread -ldl -lm" ;;
esac
target=${CARGO_TARGET_DIR:-target}
sanitize=""
if [ "${ULUA_SANITIZE:-}" = 1 ]; then
    sanitize="-fsanitize=address,undefined -fno-omit-frame-pointer -fno-sanitize-recover=undefined"
fi
cargo build --quiet -p ulua
# shellcheck disable=SC2086
cc -std=c11 -Wall -Wextra -Wpedantic -Werror $sanitize examples/run/host.c -I include \
    "$target/debug/libulua.a" -o "$out" $libs
