#!/bin/sh
# Builds "Harry Debug Switch" (68k, System 7 to Mac OS 9) with Retro68
# (ouro-tools/build-retro68.sh) into re/work/debug-switch: the application
# as MacBinary (.bin), and a disk image (.dsk) with it and README.txt
# (SimpleText, Monaco 9; also as README.txt.bin).
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
"$toolchain/bin/hmount" "$out/Harry Debug Switch.dsk" > /dev/null
"$toolchain/bin/hrename" ":HarryDebugSwitch" ":Harry Debug Switch" 2> /dev/null || true
python3 -I "$here/readme_bin.py" "$here/README.txt" "$out/README.txt.bin"
"$toolchain/bin/hcopy" -m "$out/README.txt.bin" ":README.txt"
"$toolchain/bin/hls" -l
"$toolchain/bin/humount" > /dev/null
ls -l "$out"/*.bin "$out"/*.dsk
