#!/bin/sh
# Builds "Harry Debug Switch" (68k, System 7 to Mac OS 9) with Retro68
# (ouro-tools/build-retro68.sh) into re/work/debug-switch: the application
# as MacBinary (.bin) and as a disk image (.dsk).
set -e
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
tools=${OURO_TOOLS:-$repo/../ouro-tools}
toolchain=$tools/retro68-build/toolchain
out=$repo/re/work/debug-switch
mkdir -p "$out"
cmake -S "$here" -B "$out/build" \
    -DCMAKE_TOOLCHAIN_FILE="$toolchain/m68k-apple-macos/cmake/retro68.toolchain.cmake" \
    -DCMAKE_BUILD_TYPE=Release > /dev/null
cmake --build "$out/build"
cp "$out/build/HarryDebugSwitch.bin" "$out/Harry Debug Switch.bin"
cp "$out/build/HarryDebugSwitch.dsk" "$out/Harry Debug Switch.dsk"
ls -l "$out"/*.bin "$out"/*.dsk
