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
case "$(uname -s)" in
    MINGW* | MSYS* | CYGWIN*)
        # MSVC: vswhere finds Visual Studio, and vcvars64.bat sets the SDK
        # environment for cl. The flags and the libraries match
        # crates/ulua/tests/support/host.rs.
        vswhere="C:/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe"
        vs=$("$vswhere" -latest -products '*' -property installationPath \
            -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 | tr -d '\r')
        # cygpath -w adds .exe to a path when that executable exists, so
        # convert only the directory.
        out_win="$(cygpath -w "$(dirname "$out")")\\$(basename "$out")"
        bat="$out.bat"
        cat > "$bat" <<EOF
@call "$vs\VC\Auxiliary\Build\vcvars64.bat" >nul 2>nul || exit /b 1
@cl /nologo /W4 /WX /MD /D_CRT_SECURE_NO_WARNINGS /TC /std:c11 ^
 examples\run\host.c /I include /Fo"$out_win.obj" /Fe"$out_win.exe" ^
 /link "$(cygpath -w "$target/debug/ulua.lib")" advapi32.lib bcrypt.lib ntdll.lib userenv.lib ws2_32.lib
EOF
        cmd //c "$(cygpath -w "$bat")"
        exit 0
        ;;
esac
# shellcheck disable=SC2086
cc -std=c11 -Wall -Wextra -Wpedantic -Werror $sanitize examples/run/host.c -I include \
    "$target/debug/libulua.a" -o "$out" $libs
