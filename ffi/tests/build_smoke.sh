#!/usr/bin/env bash
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
FONT="$ROOT/core/tests/assets/DejaVuSans.ttf"
EXE="$HERE/smoke.exe"
GNU_LIB="$ROOT/target/debug/librestricted_gui_ffi.a"
MSVC_LIB="$ROOT/target/debug/restricted_gui_ffi.lib"
# Probes MSVC through vswhere first, then g++/clang++. A doc once claimed the opposite order. The audit found out. It was a whole thing.
VSWHERE="/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe"

cleanup() { rm -f "$EXE" "$HERE/smoke.obj" "$HERE/smoke.o"; }
trap cleanup EXIT

cargo build -p restricted-gui-ffi --manifest-path "$ROOT/Cargo.toml"

if [ -x "$VSWHERE" ] && [ -f "$MSVC_LIB" ]; then
    VS="$("$VSWHERE" -latest -products '*' \
        -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 \
        -property installationPath)"
    MSVC_DIR="$(ls -d "$VS/VC/Tools/MSVC"/*/ | sort -V | tail -1)"
    SDK_BASE="/c/Program Files (x86)/Windows Kits/10"
    SDK_VER="$(ls "$SDK_BASE/Include" | grep -E '^10\.' | sort -V | tail -1)"
    MSVC_WIN="$(cygpath -w "${MSVC_DIR%/}")"
    SDK_WIN="$(cygpath -w "$SDK_BASE")"

    export PATH="$MSVC_DIR/bin/Hostx64/x64:$PATH"
    export INCLUDE="$MSVC_WIN\\include;$SDK_WIN\\Include\\$SDK_VER\\ucrt;$SDK_WIN\\Include\\$SDK_VER\\um;$SDK_WIN\\Include\\$SDK_VER\\shared;$SDK_WIN\\Include\\$SDK_VER\\winrt"
    export LIB="$MSVC_WIN\\lib\\x64;$SDK_WIN\\Lib\\$SDK_VER\\ucrt\\x64;$SDK_WIN\\Lib\\$SDK_VER\\um\\x64"

    (cd "$HERE" && cl //nologo //W4 //EHsc smoke.cpp \
        ../../target/debug/restricted_gui_ffi.lib \
        kernel32.lib ntdll.lib userenv.lib ws2_32.lib dbghelp.lib \
        //Fe:smoke.exe)
elif command -v g++ >/dev/null 2>&1 && [ -f "$GNU_LIB" ]; then
    g++ -std=c++17 -Wall -Wextra -Werror "$HERE/smoke.cpp" \
        "$GNU_LIB" \
        -lws2_32 -lntdll -luserenv -ldbghelp -o "$EXE"
elif command -v clang++ >/dev/null 2>&1 && [ -f "$GNU_LIB" ]; then
    clang++ -std=c++17 -Wall -Wextra -Werror "$HERE/smoke.cpp" \
        "$GNU_LIB" \
        -lws2_32 -lntdll -luserenv -ldbghelp -o "$EXE"
else
    echo "no usable C++ toolchain found (MSVC via vswhere, or g++/clang++" >&2
    echo "with a GNU-toolchain staticlib at $GNU_LIB)" >&2
    exit 1
fi

"$EXE" "$FONT"
