#!/usr/bin/env bash
# build_smoke.sh — compiles ffi/tests/smoke.cpp (a real C++ consumer of
# tacita.h) against the tacita-ffi staticlib and runs it.
#
# Compiler probe order: MSVC cl.exe (located via vswhere) first, then
# g++/clang++ — and a GNU branch is only taken when the GNU staticlib
# artifact (libtacita_ffi.a) actually exists: on the MSVC Rust
# toolchain the artifact is tacita_ffi.lib, so an installed g++
# must not win the probe and then fail on a file this workspace never
# produces. The MSVC environment is set up manually (PATH/INCLUDE/LIB)
# because vcvars64.bat is not reliable from every shell; the newest
# installed MSVC toolset and Windows 10 SDK are picked.
#
# Run from anywhere:  bash ffi/tests/build_smoke.sh
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
FONT="$ROOT/core/tests/assets/DejaVuSans.ttf"
EXE="$HERE/smoke.exe"
GNU_LIB="$ROOT/target/debug/libtacita_ffi.a"
MSVC_LIB="$ROOT/target/debug/tacita_ffi.lib"
VSWHERE="/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe"

cleanup() { rm -f "$EXE" "$HERE/smoke.obj" "$HERE/smoke.o"; }
trap cleanup EXIT

cargo build -p tacita-ffi --manifest-path "$ROOT/Cargo.toml"

if [ -x "$VSWHERE" ] && [ -f "$MSVC_LIB" ]; then
    VS="$("$VSWHERE" -latest -products '*' \
        -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 \
        -property installationPath)"
    # vswhere returns a Windows path (backslashes); globbing needs POSIX.
    VS_POSIX="$(cygpath -u "$VS")"
    MSVC_DIR="$(ls -d "$VS_POSIX/VC/Tools/MSVC"/*/ | sort -V | tail -1)"
    SDK_BASE="/c/Program Files (x86)/Windows Kits/10"
    SDK_VER="$(ls "$SDK_BASE/Include" | grep -E '^10\.' | sort -V | tail -1)"
    MSVC_WIN="$(cygpath -w "${MSVC_DIR%/}")"
    SDK_WIN="$(cygpath -w "$SDK_BASE")"

    export PATH="$MSVC_DIR/bin/Hostx64/x64:$PATH"
    export INCLUDE="$MSVC_WIN\\include;$SDK_WIN\\Include\\$SDK_VER\\ucrt;$SDK_WIN\\Include\\$SDK_VER\\um;$SDK_WIN\\Include\\$SDK_VER\\shared;$SDK_WIN\\Include\\$SDK_VER\\winrt"
    export LIB="$MSVC_WIN\\lib\\x64;$SDK_WIN\\Lib\\$SDK_VER\\ucrt\\x64;$SDK_WIN\\Lib\\$SDK_VER\\um\\x64"

    (cd "$HERE" && cl //nologo //W4 //EHsc smoke.cpp \
        ../../target/debug/tacita_ffi.lib \
        ole32.lib oleaut32.lib oleacc.lib comctl32.lib kernel32.lib user32.lib ntdll.lib userenv.lib ws2_32.lib dbghelp.lib \
        //Fe:smoke.exe)
elif command -v g++ >/dev/null 2>&1 && [ -f "$GNU_LIB" ]; then
    g++ -std=c++17 -Wall -Wextra -Werror "$HERE/smoke.cpp" \
        "$GNU_LIB" \
        -lole32 -loleaut32 -loleacc -lcomctl32 -luser32 -lws2_32 -lntdll -luserenv -ldbghelp -o "$EXE"
elif command -v clang++ >/dev/null 2>&1 && [ -f "$GNU_LIB" ]; then
    clang++ -std=c++17 -Wall -Wextra -Werror "$HERE/smoke.cpp" \
        "$GNU_LIB" \
        -lole32 -loleaut32 -loleacc -lcomctl32 -luser32 -lws2_32 -lntdll -luserenv -ldbghelp -o "$EXE"
else
    echo "no usable C++ toolchain found (MSVC via vswhere, or g++/clang++" >&2
    echo "with a GNU-toolchain staticlib at $GNU_LIB)" >&2
    exit 1
fi

"$EXE" "$FONT" "$ROOT/core/tests/assets/Noto-COLRv1.ttf"
