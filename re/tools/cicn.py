"""Decode 'cicn' colour icons (mask, 1-bit icon, 8-bit pixels) - the masks
are what the original's collision regions are made from.

cicn = PixMap (50 bytes) + mask BitMap (14) + icon BitMap (14) + data
handle (4), then mask bits, icon bits, colour table, pixel data. BitMaps
are MSB first. Usage: python3 -I cicn.py <cicn.bin> <out_prefix>
(writes _mask.png, _pixels.png).
"""
import struct
import sys


def decode(d):
    pm_row = struct.unpack(">H", d[4:6])[0] & 0x3FFF
    top, left, bottom, right = struct.unpack(">hhhh", d[6:14])
    depth = struct.unpack(">H", d[32:34])[0]
    w, h = right - left, bottom - top
    m_row = struct.unpack(">H", d[54:56])[0]
    i_row = struct.unpack(">H", d[68:70])[0]
    p = 82
    mask_bits = d[p:p + m_row * h]
    p += m_row * h
    p += i_row * h
    seed, flags, n = struct.unpack(">IHH", d[p:p + 8])
    p += 8
    ctab = {}
    for k in range(n + 1):
        v, r, g, b = struct.unpack(">HHHH", d[p:p + 8])
        ctab[k if flags & 0x8000 else v] = (r >> 8, g >> 8, b >> 8)
        p += 8
    pixels = d[p:p + pm_row * h]
    mask = [[bool(mask_bits[y * m_row + x // 8] & (0x80 >> (x % 8))) for x in range(w)] for y in range(h)]
    pix = None
    if depth == 8:
        pix = [[ctab.get(pixels[y * pm_row + x], (255, 0, 255)) for x in range(w)] for y in range(h)]
    return w, h, mask, pix, depth


if __name__ == "__main__":
    from PIL import Image
    w, h, mask, pix, depth = decode(open(sys.argv[1], "rb").read())
    im = Image.new("L", (w, h))
    im.putdata([255 if b else 0 for row in mask for b in row])
    im.save(sys.argv[2] + "_mask.png")
    if pix:
        im = Image.new("RGB", (w, h))
        im.putdata([c for row in pix for c in row])
        im.save(sys.argv[2] + "_pixels.png")
    print(w, h, depth)
