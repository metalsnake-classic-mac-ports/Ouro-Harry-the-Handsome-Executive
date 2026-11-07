#!/bin/sh
# GUI test: the game in its window, played by HARRY_SCRIPT (no accessibility
# access needed) - the menu, its options, a new game through the story, the
# question before giving up, the pause by caps lock and by focus loss, the
# options over the game, the map, F8, giving up, quitting. The pictures go
# to OUT (default: a new temporary folder) with a contact sheet
# (sheet.bmp) to look at; the settings are a fresh file there, so the
# user's are not used or changed.
# Usage: crates/harry/gui-test.sh [OUT]
set -eu
cd "$(dirname "$0")/../.."
out=${1:-$(mktemp -d)}
mkdir -p "$out"
cargo build --release -p harry
s="3:shot=01-menu 3.5:key=P 4.5:shot=02-menu-options 5:key=Escape 6:key=N"
for t in 8 9.5 11 12.5 14 15.5 17 18.5; do s="$s $t:key=Space"; done
s="$s 22:shot=03-game 23:key=Escape 23.5:shot=04-confirm 24:key=N"
s="$s 25:caps=on 25.5:shot=05-paused 26:caps=off 27:focus=lost 27.5:shot=06-focus-paused 28:key=Left_Shift"
s="$s 29:key=I+cmd 29.5:shot=07-game-options 30:key=Escape"
s="$s 31:hold=Tab 31.1:release 32:shot=08-map 32.5:hold=Tab 32.55:release 33.5:shot=09-map-closed 34:key=F8"
for t in 36 37.5 39 40.5; do s="$s $t:key=Space"; done
s="$s 43:shot=10-level-2 44:key=Escape 44.5:key=Return 48:shot=11-back-in-menu 49:key=Escape"
rm -f "$out"/*.bmp "$out/settings.txt"
HARRY_SETTINGS="$out/settings.txt" HARRY_SHOTS="$out" HARRY_SCRIPT="$s" target/release/ouro-harry
n=$(ls "$out"/[0-9]*.bmp | wc -l | tr -d ' ')
[ "$n" = 11 ] || { echo "gui-test: $n of 11 pictures" >&2; exit 1; }
python3 -I - "$out" <<'PY'
import glob, struct, sys
d = sys.argv[1]
files = sorted(glob.glob(d + "/[0-9]*.bmp"))
def read(p):
    b = open(p, "rb").read()
    off = struct.unpack_from("<I", b, 10)[0]
    w, h = struct.unpack_from("<ii", b, 18)
    rs = (w * 3 + 3) & ~3
    return [b[off + (abs(h) - 1 - y if h > 0 else y) * rs:][:w * 3] for y in range(abs(h))]
cols, W, H = 3, 320, 240
rows = (len(files) + cols - 1) // cols
TW, TH = W * cols, H * rows
out = bytearray(TW * TH * 3)
for i, f in enumerate(files):
    px, ox, oy = read(f), (i % cols) * W, (i // cols) * H
    for y in range(H):
        r = px[y * 2]
        for x in range(W):
            o = ((oy + y) * TW + ox + x) * 3
            out[o:o + 3] = r[x * 6:x * 6 + 3]
rs = (TW * 3 + 3) & ~3
data = b"".join(bytes(out[y * TW * 3:(y + 1) * TW * 3]) + b"\0" * (rs - TW * 3) for y in range(TH - 1, -1, -1))
hdr = b"BM" + struct.pack("<IHHI", 54 + len(data), 0, 0, 54) + struct.pack("<IiiHHIIiiII", 40, TW, TH, 1, 24, 0, len(data), 2835, 2835, 0, 0)
open(d + "/sheet.bmp", "wb").write(hdr + data)
PY
echo "gui-test: 11 pictures and sheet.bmp in $out"
