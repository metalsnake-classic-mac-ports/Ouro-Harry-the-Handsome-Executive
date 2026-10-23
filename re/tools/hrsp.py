"""Decode 'HrSp' sprite faces (Harry Graphics) to RGBA images.

Format (from BlitEncFaceNoClip): two Rects (top, left, bottom, right) -
the face bounds and a second rect - then 32-bit commands, high byte
= opcode, low 24 bits = n:
  0 end; 1 next row (n = encoded length of the row, unused by the
  blitter); 2 copy n pixels (data padded to a multiple of 4 bytes);
  3 skip n pixels (transparent).
Pixels are indices into the Mac's clut 8 (macpal.py).

Usage: python3 -I hrsp.py <HrSp_N.bin>... <out dir>   (writes N.png)
"""
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from macpal import clut8  # noqa: E402


def decode(d):
    """Return (rect1, rect2, rows) with rows as lists of index or None."""
    r1 = struct.unpack(">hhhh", d[0:8])
    r2 = struct.unpack(">hhhh", d[8:16])
    w, h = r1[3] - r1[1], r1[2] - r1[0]
    rows = []
    row = None
    p = 16
    while p + 4 <= len(d):
        word = struct.unpack(">I", d[p:p + 4])[0]
        p += 4
        op, n = word >> 24, word & 0xFFFFFF
        if op == 0:
            break
        if op == 1:
            row = [None] * w
            rows.append(row)
            x = 0
        elif op == 2:
            row[x:x + n] = list(d[p:p + n])
            x += n
            p += (n + 3) & ~3
        elif op == 3:
            x += n
        else:
            raise ValueError(f"opcode {op} at {p - 4}")
    while len(rows) < h:
        rows.append([None] * w)
    return r1, r2, [r[:w] for r in rows]


def to_rgba(rows):
    pal = clut8()
    out = []
    for r in rows:
        out.append(b"".join(bytes(pal[i]) + b"\xff" if i is not None else b"\0\0\0\0" for i in r))
    return out


if __name__ == "__main__":
    from PIL import Image
    *files, out = sys.argv[1:]
    os.makedirs(out, exist_ok=True)
    for f in files:
        r1, r2, rows = decode(open(f, "rb").read())
        w, h = len(rows[0]) if rows else 0, len(rows)
        im = Image.frombytes("RGBA", (w, h), b"".join(to_rgba(rows)))
        n = os.path.basename(f).split("_")[-1].split(".")[0]
        im.save(os.path.join(out, f"{n}.png"))
