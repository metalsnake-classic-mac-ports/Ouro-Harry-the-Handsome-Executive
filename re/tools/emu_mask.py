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


if __name__ == "__main__" and len(sys.argv) <= 2:
    main()


def full_chain(work="re/work"):
    """WallBounce and BGTileBounce with the real mask test: the game's mask
    tables are rebuilt from the masks (region handles at +0x1a of each
    0x2e-byte face, as InitFaces leaves them)."""
    from emu_bounce import POS_V, POS_H, CEN_V, CEN_H, FIX_H, FIX_V, SPEED_V, SPEED_H, HOT
    ms = masks(work)
    m = Machine(f"{work}/ppc")
    rnd = random.Random("fullchain")
    walls = m.alloc(64 * 0x2E)
    sp, tile, hot = m.alloc(0xA2), m.alloc(4), m.alloc(8)
    m.write(hot, struct.pack(">hhhh", *HOT))
    obstacle_face = m.alloc(0x40)
    m.w32(m.glob(0x100658D4), walls)
    m.w32(m.glob(0x100658D0), obstacle_face)
    with open(f"{work}/diff/wallchain.txt", "w") as out:
        out.write("# WallBounce + real mask test (body cicn 299, walls PICT 160): kind first tileV tileH "
                  "posV posH speedH speedV -> return atV atH posV posH speedH speedV fixH fixV\n")
        for k in range(CASES):
            kind = 4 + k % 28                                  # the kinds that use the mask test
            m.regions.clear()
            body = m.make_region(ms["body"])
            m.w32(walls + 0x1A + kind * 0x2E, m.make_region(ms[f"wall{kind}"]))
            tv, th = rnd.randint(10, 100) * 32, rnd.randint(10, 100) * 32
            pv, ph = tv - 32 + rnd.randint(-14, 46), th - 32 + rnd.randint(-14, 46)
            sh, sv = rnd.randint(-2500, 2500), rnd.randint(-2500, 2500)
            first = rnd.randint(0, 1)
            for off, v in ((POS_V, pv), (POS_H, ph), (CEN_V, pv), (CEN_H, ph), (SPEED_H, sh), (SPEED_V, sv)):
                m.w16(sp + off, v)
            m.w32(sp + FIX_H, 0x7777)
            m.w32(sp + FIX_V, 0x7777)
            m.write(tile, struct.pack(">hh", tv, th))
            r = m.call("WallBounce", sp, kind, tile, 32, 150, hot, body, first) & 0xFF
            outs = [r, m.r16(sp + CEN_V), m.r16(sp + CEN_H), m.r16(sp + POS_V), m.r16(sp + POS_H),
                    m.r16(sp + SPEED_H), m.r16(sp + SPEED_V),
                    struct.unpack(">i", m.read(sp + FIX_H, 4))[0], struct.unpack(">i", m.read(sp + FIX_V, 4))[0]]
            out.write(f"{kind} {first} {tv} {th} {pv} {ph} {sh} {sv} -> {' '.join(map(str, outs))}\n")
    print(f"wallchain: {CASES} cases")


if __name__ == "__main__" and len(sys.argv) > 2 and sys.argv[2] == "chain":
    full_chain(sys.argv[1])
