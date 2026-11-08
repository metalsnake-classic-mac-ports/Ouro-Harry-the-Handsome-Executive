"""README.txt as a SimpleText document: MacBinary II, type TEXT, creator
ttxt, CR line ends, and a 'styl' 128 resource that sets Monaco 9 so the
tables line up (without it SimpleText shows Geneva).

Usage: python3 -I readme_bin.py README.txt OUT.bin [NAME]
"""
import calendar
import struct
import sys
import time

MONACO, SIZE, HEIGHT, ASCENT = 4, 9, 11, 9
MAC_EPOCH = 2082844800  # 1904 -> 1970


def styl():
    # one ScrpSTElement from character 0: height, ascent, font, face, size, color
    return struct.pack(">hlhhhBBhHHH", 1, 0, HEIGHT, ASCENT, MONACO, 0, 0, SIZE, 0, 0, 0)


def resource_fork(rtype, rid, data):
    body = struct.pack(">I", len(data)) + data
    data_off, map_off = 256, 256 + len(body)
    types = struct.pack(">h4sHH", 0, rtype, 0, 2 + 8)  # one type, one resource, refs after the list
    refs = struct.pack(">hhI", rid, -1, 0) + struct.pack(">I", 0)  # attrs 0, data offset 0
    map_body = bytes(16) + bytes(4) + bytes(2) + bytes(2) + struct.pack(">HH", 28, 28 + len(types) + len(refs))
    rmap = map_body + types + refs
    header = struct.pack(">IIII", data_off, map_off, len(body), len(rmap))
    rmap = header + rmap[16:]
    return header + bytes(240) + body + rmap


def crc16(b):
    crc = 0
    for x in b:
        crc ^= x << 8
        for _ in range(8):
            crc = ((crc << 1) ^ 0x1021) if crc & 0x8000 else crc << 1
            crc &= 0xFFFF
    return crc


def macbinary(name, ftype, creator, data, rsrc):
    now = calendar.timegm(time.localtime()) + MAC_EPOCH  # the Mac keeps local time
    h = bytearray(128)
    n = name.encode("mac_roman")
    h[1] = len(n)
    h[2:2 + len(n)] = n
    h[65:69], h[69:73] = ftype, creator
    struct.pack_into(">II", h, 83, len(data), len(rsrc))
    struct.pack_into(">II", h, 91, now, now)
    h[122], h[123] = 129, 129
    struct.pack_into(">H", h, 124, crc16(h[:124]))
    pad = lambda b: b + bytes(-len(b) % 128)
    return bytes(h) + pad(data) + pad(rsrc)


def main():
    src, out = sys.argv[1], sys.argv[2]
    name = sys.argv[3] if len(sys.argv) > 3 else "README.txt"
    text = open(src, encoding="ascii").read().replace("\n", "\r").encode("mac_roman")
    with open(out, "wb") as f:
        f.write(macbinary(name, b"TEXT", b"ttxt", text, resource_fork(b"styl", 128, styl())))


if __name__ == "__main__":
    main()
