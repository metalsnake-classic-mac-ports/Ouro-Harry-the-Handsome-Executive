"""Record the water cooler (500-501): SetupCoolerSprite, frames of
HandleCoolerSprite, then HitCoolerSprite with Harry (kicking: the kick
rect; walking: the mask test with his body mask and the cooler's mask,
HrSp 2002 as BuildMaskRegion makes it) or with a shot / effect.

Each line: "<name=value ...> -> <cooler dump> | <name=value ...>"
  other   1 Harry, 2 player shot, 3 enemy shot, 4 effect, 5 effect of
          kind 9610, 6 another sprite
  events  p:<snd>, 3d:<snd>, x:<h>:<v>:<a>:<b> (EasyExplosion)
Usage: python3 -I emu_cooler.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from sprites import Faces, tvectors, dump  # noqa: E402
from emu_mask import masks  # noqa: E402

TV = {1: 0x10066FC4, 2: 0x100672FC, 3: 0x100671BC, 4: 0x10067194, 5: 0x10067194, 6: 0x1006702C}
BODY_FACE, COOLER_MASK_FACE, CONTACT = 0x10065EA0, 0x1006600C, 0x10065E1C
SOUNDS = {0x10065C04: 501, 0x10065C00: 502, 0x10065C08: 500}
CASES = 12000


def s16(v):
    return ((v & 0xFFFF) ^ 0x8000) - 0x8000


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m)
    m.call("InitCoolerSprite")
    m.call("InitPlayerKickRects")
    tv = tvectors(f"{work}/ppc")
    lvl = m.load_level(open(f"{work}/res/Episode_1/Hlvl_1.bin", "rb").read())
    ms = masks(work)
    lines = open(f"{work}/diff/facemasks.txt").read().splitlines()
    k = next(i for i, ln in enumerate(lines) if ln.startswith("= 2002 "))
    cooler_mask = {(x, y) for y in range(32) for x in range(32) if lines[k + 1 + y][x] == "#"}
    ev = []
    m.stub_routine("STPlayRegSound", lambda mm: ev.append("p:%d" % mm.get(3)) or 0)
    m.stub_routine("STPlay3DSound", lambda mm: ev.append("3d:%d" % mm.get(3)) or 0)
    m.stub_routine("EasyExplosion", lambda mm: ev.append("x:%d:%d:%d:%d" % tuple(s16(mm.get(i)) for i in (3, 4, 7, 8))) or 0)
    for slot, snd in SOUNDS.items():
        m.w32(m.glob(slot), snd)
    body_face = m.alloc(0x40)
    m.w32(m.glob(BODY_FACE), body_face)
    mask_face = m.r32(m.glob(COOLER_MASK_FACE))
    contact = m.glob(CONTACT)
    sp, other = m.alloc(0xA2), m.alloc(0xA2)
    mark = m.heap
    rnd = random.Random("cooler")
    out = open(f"{work}/diff/cooler.txt", "w")
    out.write("# cooler: name=value inputs -> cooler dump | name=value outputs\n")
    for case in range(CASES):
        m.heap = mark
        m.regions.clear()
        m.w32(body_face + 0x1A, m.make_region(ms["body"]))
        m.w32(mask_face + 0x14, m.make_region(cooler_mask))
        cv, ch = rnd.randint(200, 3800), rnd.randint(200, 3800)
        st = {"kind": rnd.choice([500, 501]), "cv": cv, "ch": ch, "dir": rnd.choice([1, 1, 2, 3, 4, rnd.randint(0, 12)]),
              "frames": rnd.choice([0, 0, 1, 2, rnd.randint(0, 12)]), "levelindex": rnd.choice([-1, rnd.randint(0, 255)]),
              "info1": rnd.choice([0, 0, 3, 7]), "frozen": int(rnd.random() < 0.03),
              "other": rnd.choice([1, 1, 1, 1, 2, 3, 4, 5, 6]), "ofrozen": int(rnd.random() < 0.05),
              "oair": rnd.choice([0, 0, 0, 0, 3]), "kicking": int(rnd.random() < 0.4), "odir": rnd.randint(0, 35),
              "oav": cv + rnd.randint(-60, 30), "oah": ch + rnd.randint(-60, 30),
              "ospeedv": rnd.randint(-2500, 2500), "ospeedh": rnd.randint(-2500, 2500), "contact": rnd.choice([0, 64])}
        m.write(sp, bytes(0xA2))
        m.w16(sp, st["kind"])
        m.w16(sp + 2, cv)
        m.w16(sp + 4, ch)
        m.call("SetupCoolerSprite", sp)
        m.w16(sp + 0x66, st["dir"])
        m.w16(sp + 0x68, st["levelindex"])
        if st["levelindex"] >= 0:
            m.write(lvl + 0x10000 + st["levelindex"] * 16, b"\1")
            m.w16(lvl + 0x10006 + st["levelindex"] * 16, st["info1"])
        ev.clear()
        for f in range(st["frames"]):
            m.call("HandleCoolerSprite", sp)
        m.write(sp + 0x92, bytes([st["frozen"]]))
        m.write(other, bytes(0xA2))
        m.w16(other, 9610 if st["other"] == 5 else 0)
        m.w32(other + 0x1A, TV[st["other"]])
        m.write(other + 0x92, bytes([st["ofrozen"]]))
        for off, n in ((0x8E, "oair"), (0x66, "odir"), (0x5A, "oav"), (0x5C, "oah"), (0x02, "oav"), (0x04, "oah"),
                       (0x4E, "ospeedv"), (0x50, "ospeedh")):
            m.w16(other + off, st[n])
        m.w32(other + 0x5E, 0x7777)
        m.w32(other + 0x62, 0x7777)
        m.write(other + 6, struct.pack(">hhhh", *((0, 0, 64, 64) if st["kicking"] else (22, 22, 42, 42))))
        m.w16(contact, st["contact"])
        m.call("HitCoolerSprite", sp, other)
        rec = m.read(lvl + 0x10000 + st["levelindex"] * 16, 1)[0] if st["levelindex"] >= 0 else 9
        outs = {"present": rec, "contact": m.r16(contact), "ofrozen": m.read(other + 0x92, 1)[0],
                "opv": m.r16(other + 2), "oph": m.r16(other + 4), "oav": m.r16(other + 0x5A), "oah": m.r16(other + 0x5C),
                "ofixh": struct.unpack(">i", m.read(other + 0x5E, 4))[0],
                "ofixv": struct.unpack(">i", m.read(other + 0x62, 4))[0],
                "ospeedv": m.r16(other + 0x4E), "ospeedh": m.r16(other + 0x50), "events": ",".join(ev) or "-"}
        fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())
        out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} | {fmt(outs)}\n")
    out.close()
    print(f"cooler: {CASES} cases")


if __name__ == "__main__":
    main()
