"""Record the stairs (9900-9909): SetupStairsSprite for each kind (and the
door kinds it knows), then HandleStairsCollision(sprite, half, hot,
stairs) for sprites coming at them from every side - Harry (half 32, his
hot rect) and objects (half 12 / 18, their own).

Each line: "<name=value ...> -> <stairs dump> | <sprite outputs>"
Usage: python3 -I emu_stairs.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from sprites import Faces, tvectors, dump  # noqa: E402

PHYSICS = 0x100659D8
KINDS = [9900, 9901, 9902, 9903, 9904, 9905, 922, 923, 924, 931, 932, 933, 934]
CASES = 30000


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m)
    m.call("InitStairsSprite")
    tv = tvectors(f"{work}/ppc")
    physics = m.glob(PHYSICS)
    for i, v in enumerate((50, 70, 245, 252, 150)):
        m.w16(physics + 2 * i, v)
    sp, stairs, hot = m.alloc(0xA2), m.alloc(0xA2), m.alloc(8)
    rnd = random.Random("stairs")
    fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())
    out = open(f"{work}/diff/stairs.txt", "w")
    out.write("# HandleStairsCollision: name=value inputs -> stairs dump | sprite outputs\n")
    for _ in range(CASES):
        kind = rnd.choice(KINDS[1:5] * 3 + KINDS)
        sv, sh = rnd.randint(500, 3500), rnd.randint(500, 3500)
        half = rnd.choice([32, 32, 12, 18])
        hr = (22, 22, 42, 42) if half == 32 else rnd.choice([(0, 0, 24, 24), (8, 8, 28, 28)])
        st = {"kind": kind, "sv": sv, "sh": sh, "half": half, "hot": ":".join(map(str, hr)),
              "v": sv + rnd.randint(-50, 50) - half + 16, "h": sh + rnd.randint(-50, 50) - half + 16,
              "speedv": rnd.choice([0, rnd.randint(-100, 100), rnd.randint(-900, 900)]),
              "speedh": rnd.choice([0, rnd.randint(-100, 100), rnd.randint(-900, 900)]),
              "air": rnd.choice([0, 0, 5]), "flip": rnd.choice([0, 0x26AF]), "bounce": rnd.choice([150, 100, 255])}
        st["pv"], st["ph"] = st["v"] + rnd.choice([0, 0, rnd.randint(-3, 3)]), st["h"] + rnd.choice([0, 0, rnd.randint(-3, 3)])
        st["fixh"] = st["ph"] * 256 + rnd.choice([0, rnd.randint(0, 255)])
        st["fixv"] = st["pv"] * 256 + rnd.choice([0, rnd.randint(0, 255)])
        m.w16(physics + 8, st["bounce"])
        m.write(stairs, bytes(0xA2))
        m.w16(stairs, kind)
        m.w16(stairs + 2, sv)
        m.w16(stairs + 4, sh)
        m.call("SetupStairsSprite", stairs)
        m.write(sp, bytes(0xA2))
        m.w16(sp, 69 if half == 32 else 601)
        for off, k in ((2, "pv"), (4, "ph"), (0x5A, "v"), (0x5C, "h"), (0x4E, "speedv"), (0x50, "speedh"),
                       (0x8E, "air"), (0x90, "flip")):
            m.w16(sp + off, st[k])
        m.w32(sp + 0x5E, st["fixh"])
        m.w32(sp + 0x62, st["fixv"])
        m.write(hot, struct.pack(">hhhh", *hr))
        m.call("HandleStairsCollision", sp, half, hot, stairs)
        o = {k: struct.unpack(">h", m.read(sp + off, 2))[0] for k, off in (
            ("pv", 2), ("ph", 4), ("v", 0x5A), ("h", 0x5C), ("speedv", 0x4E), ("speedh", 0x50), ("air", 0x8E), ("flip", 0x90))}
        o["fixh"] = struct.unpack(">i", m.read(sp + 0x5E, 4))[0]
        o["fixv"] = struct.unpack(">i", m.read(sp + 0x62, 4))[0]
        out.write(f"{fmt(st)} -> {dump(m, stairs, faces, tv)} | {fmt(o)}\n")
    out.close()
    print(f"stairs: {CASES} cases")


if __name__ == "__main__":
    main()
