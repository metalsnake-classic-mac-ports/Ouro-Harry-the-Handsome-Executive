"""Record the mail guy (1601-1631): SetupMailGuySprite at the mail guys'
places in Episode 1 (and nearby), then frames of HandleMailGuySprite each
followed by CheckTileCollisions with it as the only sprite
(HitMailGuyTileSprite) - running, stopping at walls, turning round.

Each line: "<name=value ...> -> <dump> | entry=<id>:<info0>:<info1> events=<per frame, ';'>"
  events  dyn:<snd>:<volume>:<at v>:<at h>:<speed v>:<speed h> (STPlayDynamicSound)
Usage: python3 -I emu_mailguy.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from sprites import Faces, tvectors, dump  # noqa: E402

SAT, LEVEL_H = 0x10065B70, 0x10065B64
SOUNDS = {0x10065C10: 400}
RUNS = 6000


def s16(x):
    x &= 0xFFFF
    return x - 0x10000 if x & 0x8000 else x


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m, work)
    m.call("InitMailGuySprite")
    m.call("InitTileHotRects")
    tv = tvectors(f"{work}/ppc")
    ev = []
    m.stub_routine("STPlayDynamicSound", lambda mm: ev.append("dyn:%d:%d:%d:%d:%d:%d" % (
        mm.get(3), s16(mm.get(5)), s16(mm.get(6) >> 16), s16(mm.get(6)), s16(mm.get(7) >> 16), s16(mm.get(7)))) or 0)
    for slot, snd in SOUNDS.items():
        m.w32(m.glob(slot), snd)
    sat = m.glob(SAT)
    sp = m.alloc(0xA2)
    rnd = random.Random("mailguy")
    fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())
    out = open(f"{work}/diff/mailguy.txt", "w")
    out.write("# mail guy: name=value inputs -> dump | entry events\n")
    places = []
    for lvl in range(1, 16):
        data = open(f"{work}/res/Episode_1/Hlvl_{lvl}.bin", "rb").read()
        for i in range(256):
            e = data[0x10000 + i * 16:0x10000 + i * 16 + 16]
            used, kind = e[0], struct.unpack(">h", e[2:4])[0]
            if used and kind in (1601, 1611, 1621, 1631):
                v, h = struct.unpack(">hh", e[12:16])
                places.append((lvl, i, kind, v - 6, h - 6))
    levels = {}
    n = 0
    for _ in range(RUNS):
        lvl, index, kind, v, h = rnd.choice(places)
        if lvl not in levels:
            data = open(f"{work}/res/Episode_1/Hlvl_{lvl}.bin", "rb").read()
            m.load_level(data)
            levels[lvl] = (data, m.r32(m.glob(LEVEL_H)))
        data, handle = levels[lvl]
        base = m.r32(handle)
        if m.r32(m.glob(LEVEL_H)) != handle:
            m.w32(m.glob(LEVEL_H), handle)
            m.call("LoadTileDefinitions")
        m.write(base + 0x10000, data[0x10000:0x11000])
        st = {"level": lvl, "index": index, "kind": rnd.choice([kind, kind, 1601, 1611, 1621, 1631]),
              "v": v + rnd.choice([0, 0, rnd.randint(-200, 200)]), "h": h + rnd.choice([0, 0, rnd.randint(-200, 200)]),
              "info0": rnd.choice([0, 0, 0, 3, -4, 6]), "info1": rnd.choice([0, 0, 0, 5, 40, 41, 80]),
              "frozen": int(rnd.random() < 0.02), "frames": rnd.randint(1, 150), "seed": rnd.randint(1, 0x7FFFFFFE)}
        e = base + 0x10000 + index * 16
        m.w16(e + 4, st["info0"])
        m.w16(e + 6, st["info1"])
        m.write(sp, bytes(0xA2))
        m.w16(sp, st["kind"])
        m.w16(sp + 2, st["v"])
        m.w16(sp + 4, st["h"])
        m.call("SetupMailGuySprite", sp)
        m.w16(sp + 0x68, index)
        m.write(sp + 0x92, bytes([st["frozen"]]))
        m.rand_seed = st["seed"]
        evs = []
        for _f in range(st["frames"]):
            ev.clear()
            m.call("HandleMailGuySprite", sp)
            m.w32(sat + 0x78, sp)
            m.w32(sp + 0x2E, 0)
            m.call("CheckTileCollisions")
            evs.append(",".join(ev) or "-")
        o = f"entry={s16(m.r16(e + 2))}:{s16(m.r16(e + 4))}:{s16(m.r16(e + 6))} events={';'.join(evs)}"
        out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} | {o}\n")
        n += 1
    out.close()
    print(f"mailguy: {n} cases")


if __name__ == "__main__":
    main()
