#!/bin/sh
# Rebuild the Ghidra project of the PowerPC build and decompile everything.
# Usage: re/ghidra/run.sh            (from the repository root)
# Needs: re/work/app/pef.bin (data fork of "Harry 1.0.1"), Ghidra (brew),
# re/work/venv with capstone. Output in re/work/ppc/:
#   traceback.txt   routine starts and names (pef_traceback.py)
#   functions.txt   all functions after naming
#   toc_functions.txt TOC slots that lead to a function
#   data.bin        the unpacked data section (4-byte start address first)
#   harry.c         decompiled C of every function
set -e
R="$(cd "$(dirname "$0")/../.." && pwd)"
W="$R/re/work"
G=/opt/homebrew/opt/ghidra/libexec/support/analyzeHeadless
mkdir -p "$W/ppc" "$W/ghidra-proj"
cp "$W/app/pef.bin" "$W/ppc/Harry.pef"
"$W/venv/bin/python" -I "$R/re/tools/pef_traceback.py" "$W/ppc/Harry.pef" > "$W/ppc/traceback.txt"
"$G" "$W/ghidra-proj" harry-ppc -import "$W/ppc/Harry.pef" -overwrite \
    -processor PowerPC:BE:32:default -cspec default \
    -scriptPath "$R/re/ghidra" \
    -postScript ApplyNames.java "$W/ppc/traceback.txt" \
    -postScript NopTOCRestore.java \
    -postScript SetTOC.java \
    -postScript NameTOC.java "$W/ppc/toc_functions.txt" \
    -postScript ApplyLabels.java "$R/re/ghidra/globals.txt" \
    -postScript ApplyStruct.java "$R/re/ghidra/sprite.txt" 0xa2 \
    -postScript ExportFunctions.java "$W/ppc/functions.txt" \
    -postScript DumpData.java "$W/ppc/data.bin" \
    -postScript DecompileAll.java "$W/ppc/harry.c" > "$W/ppc/ghidra.log" 2>&1
grep -E "ApplyNames|NopTOC|SetTOC|NameTOC|ApplyLabels|ApplyStruct|DumpData|ERROR" "$W/ppc/ghidra.log"
