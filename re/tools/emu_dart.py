"""Record the dart enemy (1011 / 1021 / 1031 / 1041): SetupDartEnemySprite,
frames of HandleDartEnemySprite (ThrowDart with SATNewSprite stubbed and
logged) and HitDartEnemySprite at one frame with a player shot, another
sprite or a frozen shot.

Each line: "<name=value ...> -> <dump> | favor=<..> ofrozen=<..> events=<...>"
  events  3d:<snd>, n:<kind>:<h>:<v>:<speed v>:<speed h>
Usage: python3 -I emu_dart.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from sprites import Faces, tvectors, dump  # noqa: E402

VIEW, FAVOR = 0x100659BC, 0x1006856E
SOUNDS = {0x10065C18: 300, 0x10065BB0: 4700}
TV = {"playershot": 0x100672FC, "bonus": 0x1006702C}
CASES = 15000


def s16(v):
    return ((v & 0xFFFF) ^ 0x8000) - 0x8000


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m)
    m.call("InitDartEnemySprite")
    tv = tvectors(f"{work}/ppc")
    data = open(f"{work}/res/Episode_1/Hlvl_1.bin", "rb").read()
    lvl = m.load_level(data)
    ev = []
    m.stub_routine("STPlay3DSound", lambda mm: ev.append("3d:%d" % mm.get(3)) or 0)

    def new_sprite(mm):
        rec = mm.alloc(0xA2)
        ev.append(("n", rec, s16(mm.get(3)), s16(mm.get(4)), s16(mm.get(5))))
        return rec
    m.stub_routine("SATNewSprite", new_sprite)
    for slot, snd in SOUNDS.items():
        m.w32(m.glob(slot), snd)
    view = m.glob(VIEW)
    sp, other = m.alloc(0xA2), m.alloc(0xA2)
    mark = m.heap
    rnd = random.Random("dart")
    out = open(f"{work}/diff/dart.txt", "w")
    out.write("# dart enemy: name=value inputs -> dump | outputs\n")
    for case in range(CASES):
        m.heap = mark
        m.write(lvl + 0x10000, data[0x10000:0x11000])
        kind = rnd.choice([1011, 1021, 1031, 1041])
        v, h = rnd.randint(100, 3800), rnd.randint(100, 3800)
        st = {"kind": kind, "h": h, "v": v, "seed": rnd.randint(1, 0x7FFFFFFE),
              "viewv": v - rnd.choice([rnd.randint(0, 448), rnd.randint(-600, 600), -64, -65, 511, 512]),
              "viewh": h - rnd.choice([rnd.randint(0, 448), rnd.randint(-600, 600), -64, -65, 511, 512]),
              "levelindex": rnd.choice([-1, rnd.randint(0, 255), rnd.randint(0, 255)]),
              "info0": rnd.choice([0, 0, 1, 1, 2, -1]),
              "settimer": rnd.choice([-9999, -9999, rnd.randint(0, 5), rnd.randint(0, 140)]),
              "setf6e": rnd.choice([-9999, -9999, rnd.randint(0, 50), 19, 44, 45, 46]),
              "setlayer": rnd.choice([-9999, -9999, 10]),
              "frames": rnd.choice([1, 2, rnd.randint(0, 40), rnd.randint(0, 200)]),
              "frozenframe": rnd.choice([-1, -1, -1, rnd.randint(0, 40)]), "hitframe": -1,
              "other": rnd.choice(["playershot", "playershot", "bonus", "frozenshot"]), "favor": rnd.randint(0, 144)}
        st["hitframe"] = rnd.choice([-1, rnd.randint(0, st["frames"])])
        if st["levelindex"] >= 0:
            m.w16(lvl + 0x10004 + st["levelindex"] * 16, st["info0"])
        m.write(view, struct.pack(">hh", st["viewv"], st["viewh"]))
        m.w16(FAVOR, st["favor"])
        m.rand_seed = st["seed"]
        m.write(sp, bytes(0xA2))
        m.w16(sp, kind)
        m.w16(sp + 2, v)
        m.w16(sp + 4, h)
        m.call("SetupDartEnemySprite", sp)
        m.w16(sp + 0x68, st["levelindex"])
        if st["settimer"] != -9999:
            m.w16(sp + 0x74, st["settimer"])
        if st["setf6e"] != -9999:
            m.w32(sp + 0x6E, st["setf6e"])
        if st["setlayer"] != -9999:
            m.w16(sp + 0x4C, st["setlayer"])
        m.write(other, bytes(0xA2))
        m.w32(other + 0x1A, TV["bonus" if st["other"] == "bonus" else "playershot"])
        m.write(other + 0x92, b"\1" if st["other"] == "frozenshot" else b"\0")
        ev.clear()
        for f in range(st["frames"] + 1):
            if f == st["frozenframe"]:
                m.write(sp + 0x92, b"\1")
            if f == st["hitframe"]:
                m.call("HitDartEnemySprite", sp, other)
            if f < st["frames"]:
                m.call("HandleDartEnemySprite", sp)
                for j, e in enumerate(ev):
                    if isinstance(e, tuple):
                        _, rec, k, nh, nv = e
                        ev[j] = f"n:{k}:{nh}:{nv}:{m.r16(rec + 0x4E)}:{m.r16(rec + 0x50)}"
        fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())
        out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} | favor={m.r16(FAVOR)} "
                  f"ofrozen={m.read(other + 0x92, 1)[0]} events={','.join(ev) or '-'}\n")
    out.close()
    print(f"dart: {CASES} cases")


if __name__ == "__main__":
    main()
