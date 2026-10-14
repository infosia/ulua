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
cargo build --quiet -p ulua
# shellcheck disable=SC2086
cc -std=c11 -Wall -Wextra -Wpedantic -Werror examples/run/host.c -I include \
    target/debug/libulua.a -o "$out" $libs
