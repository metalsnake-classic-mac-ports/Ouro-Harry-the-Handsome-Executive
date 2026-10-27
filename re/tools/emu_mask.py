"""Record the original's pixel-exact mask test (FUN_10054124 with
FUN_10053eac = 68k REGIONSEPARATE) on the real masks.

Regions are pixel sets (ppcmachine's QuickDraw region stubs). Masks:
Harry's collision body (cicn 299, as HitPlayerTileSprite passes it to
WallBounce / BGTileBounce), the 32 wall shapes (application PICT 160,
8 x 4 tiles of 32 x 32, as FUN_10054a14 cuts them) and the background
obstacle (cicn 290). Writes re/work/diff/masks/<name>.txt (rows of # and .)
and re/work/diff/masktest.txt: tile posV posH -> hit posV posH.

Usage: python3 -I emu_mask.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from cicn import decode as cicn_decode  # noqa: E402
from render_level import pict_bits  # noqa: E402

MASK_TEST = 0x10054124
CASES = 12000


def masks(work):
    out = {}
    w, h, m, _, _ = cicn_decode(open(f"{work}/res/Harry_Graphics/cicn_299.bin", "rb").read())
    out["body"] = {(x, y) for y in range(h) for x in range(w) if m[y][x]}
    w, h, m, _, _ = cicn_decode(open(f"{work}/res/Harry_Graphics/cicn_290.bin", "rb").read())
    out["obstacle"] = {(x, y) for y in range(h) for x in range(w) if m[y][x]}
    sheet = pict_bits(f"{work}/app/rsrc/PICT_160.bin")
    for k in range(32):
        ox, oy = (k % 8) * 32, (k // 8) * 32
        out[f"wall{k}"] = {(x, y) for y in range(32) for x in range(32) if sheet.getpixel((ox + x, oy + y))}
    return out


def write_mask(path, px, w, h):
    with open(path, "w") as f:
        for y in range(h):
            f.write("".join("#" if (x, y) in px else "." for x in range(w)) + "\n")


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    out_dir = f"{work}/diff"
    os.makedirs(f"{out_dir}/masks", exist_ok=True)
    ms = masks(work)
    for name, px in ms.items():
        size = 64 if name == "body" else 32
        write_mask(f"{out_dir}/masks/{name}.txt", px, size, size)
    m = Machine(f"{work}/ppc")
    rnd = random.Random("masktest")
    sp = m.alloc(0xA2)
    tiles = [n for n in ms if n != "body"]
    with open(f"{out_dir}/masktest.txt", "w") as out:
        out.write("# FUN_10054124 + REGIONSEPARATE, sprite mask 'body' at pos, tile mask at (tileV, tileH): "
                  "tile tileV tileH posV posH -> hit posV posH\n")
        for k in range(CASES):
            name = tiles[k % len(tiles)]
            tv, th = rnd.randint(10, 100) * 32, rnd.randint(10, 100) * 32
            pv, ph = tv - 32 + rnd.randint(-14, 46), th - 32 + rnd.randint(-14, 46)
            m.regions.clear()
            spr = m.make_region(ms["body"])
            til = m.make_region(ms[name])
            m.w16(sp + 2, pv)
            m.w16(sp + 4, ph)
            r = m.call(MASK_TEST, sp, ((tv & 0xFFFF) << 16) | (th & 0xFFFF), 0x28, spr, til) & 0xFF
            out.write(f"{name} {tv} {th} {pv} {ph} -> {r} {m.r16(sp + 2)} {m.r16(sp + 4)}\n")
    print(f"masktest: {CASES} cases, {len(ms)} masks")


if __name__ == "__main__":
    main()
