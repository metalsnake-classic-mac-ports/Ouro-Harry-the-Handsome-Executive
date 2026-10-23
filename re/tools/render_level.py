"""Render a Harry level (Hlvl) to PNG as a format check.

Usage: python3 -I render_level.py <res dir> <level id> <out.png> [scale]
<res dir> is re/work/res (Episode_1/, Harry_Graphics/ from macres.py dump).
Tile sets: level 0x120fe = background PICT (96 tiles, 8 per row), 0x120fc =
foreground PICT (64 tiles) with its 1-bit mask at id + 1; 0 = the default
sets 180 / 181 (+182) in Harry Graphics. Grids are column major (x * 128 +
y). Sprites from the level's list are marked with a dot and their id.
"""
import os
import struct
import sys

from PIL import Image, ImageDraw

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.expanduser("~/.claude/skills/classic-mac-port/scripts"))
from pict import decode, unpack_bits  # noqa: E402


def pict_rgb(path):
    w, h, rows, pal = decode(open(path, "rb").read())
    return Image.frombytes("RGB", (w, h), b"".join(b"".join(bytes(pal[i]) for i in r) for r in rows))


def pict_bits(path):
    """1-bit PICT v2 (PackBitsRect with a BitMap) -> 'L' image, 255 = set bit."""
    p = open(path, "rb").read()
    op = p.index(b"\x00\x98", 10)
    row_bytes = struct.unpack(">H", p[op + 2:op + 4])[0] & 0x3FFF
    top, left, bottom, right = struct.unpack(">hhhh", p[op + 4:op + 12])
    w, h = right - left, bottom - top
    pos = op + 12 + 8 + 8 + 2
    out = bytearray()
    for _ in range(h):
        if row_bytes > 250:
            n = struct.unpack(">H", p[pos:pos + 2])[0]
            pos += 2
        else:
            n = p[pos]
            pos += 1
        line, _ = unpack_bits(p[pos:pos + n], row_bytes)
        pos += n
        for x in range(w):
            out.append(255 if line[x >> 3] & (0x80 >> (x & 7)) else 0)
    return Image.frombytes("L", (w, h), bytes(out))


def find_pict(res, pid):
    for d in ("Episode_1", "Harry_Graphics", "Harry_Titles"):
        f = os.path.join(res, d, f"PICT_{pid}.bin")
        if os.path.exists(f):
            return f
    raise FileNotFoundError(pid)


def main():
    res, lvl, out = sys.argv[1], int(sys.argv[2]), sys.argv[3]
    scale = float(sys.argv[4]) if len(sys.argv) > 4 else 1.0
    d = open(os.path.join(res, "Episode_1", f"Hlvl_{lvl}.bin"), "rb").read()
    w16 = lambda o: struct.unpack(">h", d[o:o + 2])[0]
    fg_id, bg_id = w16(0x120FC) or 181, w16(0x120FE) or 180
    bg = pict_rgb(find_pict(res, bg_id))
    fg = pict_rgb(find_pict(res, fg_id))
    mask = pict_bits(find_pict(res, fg_id + 1))
    img = Image.new("RGB", (4096, 4096))
    for x in range(128):
        for y in range(128):
            b = struct.unpack(">H", d[(x * 128 + y) * 2:(x * 128 + y) * 2 + 2])[0]
            if b < 96:
                img.paste(bg.crop(((b % 8) * 32, (b // 8) * 32, (b % 8) * 32 + 32, (b // 8) * 32 + 32)), (x * 32, y * 32))
            f = struct.unpack(">H", d[0x8000 + (x * 128 + y) * 2:0x8000 + (x * 128 + y) * 2 + 2])[0]
            if f < 64:
                box = ((f % 8) * 32, (f // 8) * 32, (f % 8) * 32 + 32, (f // 8) * 32 + 32)
                img.paste(fg.crop(box), (x * 32, y * 32), mask.crop(box))
    dr = ImageDraw.Draw(img)
    for k in range(256):
        rec = d[0x10000 + 16 * k:0x10010 + 16 * k]
        if rec[0] and struct.unpack(">h", rec[2:4])[0]:
            oid = struct.unpack(">h", rec[2:4])[0]
            v, h = struct.unpack(">hh", rec[12:16])
            dr.ellipse((h - 4, v - 4, h + 4, v + 4), outline=(255, 0, 255), width=2)
            dr.text((h + 6, v - 6), str(oid), fill=(255, 0, 255))
    sh, sv = w16(0x120F4), w16(0x120F6)
    dr.rectangle((sh - 6, sv - 6, sh + 6, sv + 6), outline=(0, 255, 0), width=3)
    if scale != 1.0:
        img = img.resize((int(4096 * scale), int(4096 * scale)), Image.LANCZOS)
    img.save(out)
    name = d[0x114C1:0x114C1 + d[0x114C0]].decode("mac_roman")
    print(f"level {lvl} '{name}': bg PICT {bg_id}, fg PICT {fg_id}")


if __name__ == "__main__":
    main()
