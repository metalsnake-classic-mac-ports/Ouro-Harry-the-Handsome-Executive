"""Record the ramps (9701-9704).

collide: HandleRampCollision(sprite, half, hot rect, ramp, sound) with a
sprite (Harry, kind 69, half 32 and his hot rect, or a plant-like sprite,
half 18) all around a ramp made by SetupRampSprite.
  line: "<name=value ...> -> <ramp dump> | pv ph fixh fixv speedv speedh air f94 sound"

task: SetupRampSprite and frames of HandleRampSprite on level 1 (entry
info #1 for pushable ramps; HandleGlide's floor from the level).
  line: "<name=value ...> -> <dump>"
Usage: python3 -I emu_ramp.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from sprites import Faces, tvectors, dump  # noqa: E402

PHYSICS = 0x100659D8
KINDS = [9701, 9702, 9703, 9704]
COLLIDES, TASKS = 40000, 10000


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m)
    m.stub_routine(0x1005F340, lambda mm: mm.alloc(0x40))
    m.call("InitRampSprite")
    tv = tvectors(f"{work}/ppc")
    data = open(f"{work}/res/Episode_1/Hlvl_1.bin", "rb").read()
    lvl = m.load_level(data)
    snd = []
    m.stub_routine("STPlayRegSound", lambda mm: snd.append(mm.get(3)) or 0)
    physics = m.glob(PHYSICS)
    for i, v in enumerate((50, 70, 245, 252, 150)):
        m.w16(physics + 2 * i, v)
    sp, ramp, hot = m.alloc(0xA2), m.alloc(0xA2), m.alloc(8)
    rnd = random.Random("ramp")
    fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())
    out = open(f"{work}/diff/rampcollide.txt", "w")
    out.write("# HandleRampCollision: name=value inputs -> ramp dump | sprite outputs\n")
    for _ in range(COLLIDES):
        kind = rnd.choice(KINDS)
        rv, rh = rnd.randint(500, 3500), rnd.randint(500, 3500)
        player = rnd.random() < 0.6
        half = 32 if player else 18
        hr = (22, 22, 42, 42) if player else rnd.choice([(0, 0, 36, 36), (4, 4, 32, 32)])
        st = {"kind": kind, "rv": rv, "rh": rh, "rfixh": (rh + rnd.choice([0, 0, rnd.randint(-3, 3)])) * 256 + rnd.choice([0, rnd.randint(0, 255)]),
              "rfixv": (rv + rnd.choice([0, 0, rnd.randint(-3, 3)])) * 256 + rnd.choice([0, rnd.randint(0, 255)]),
              "rspeedv": rnd.choice([0, rnd.randint(-500, 500)]), "rspeedh": rnd.choice([0, rnd.randint(-500, 500)]),
              "player": int(player), "half": half, "hot": ":".join(map(str, hr)),
              "pv": rv + rnd.randint(-60, 70) - half + 12, "ph": rh + rnd.randint(-60, 70) - half + 12,
              "speedv": rnd.choice([0, rnd.randint(-100, 100), rnd.randint(-900, 900), rnd.choice([-451, -450, 450, 451, 452])]),
              "speedh": rnd.choice([0, rnd.randint(-100, 100), rnd.randint(-900, 900), rnd.choice([-451, -450, 450, 451, 452])]),
              "air": rnd.choice([0, 0, 0, 5]), "f94": rnd.choice([0, 0, 0, 0, 1, 3, -1]),
              "bounce": rnd.choice([150, 100, 255])}
        st["spv"], st["sph"] = st["pv"] + rnd.randint(-2, 2), st["ph"] + rnd.randint(-2, 2)
        st["fixh"] = st["ph"] * 256 + rnd.choice([0, rnd.randint(0, 255)])
        st["fixv"] = st["pv"] * 256 + rnd.choice([0, rnd.randint(0, 255)])
        m.w16(physics + 8, st["bounce"])
        m.write(ramp, bytes(0xA2))
        m.w16(ramp, kind)
        m.w16(ramp + 2, rv)
        m.w16(ramp + 4, rh)
        m.call("SetupRampSprite", ramp)
        m.w32(ramp + 0x5E, st["rfixh"])
        m.w32(ramp + 0x62, st["rfixv"])
        m.w16(ramp + 0x4E, st["rspeedv"])
        m.w16(ramp + 0x50, st["rspeedh"])
        m.write(sp, bytes(0xA2))
        m.w16(sp, 69 if player else 601)
        m.w16(sp + 2, st["spv"])
        m.w16(sp + 4, st["sph"])
        m.w32(sp + 0x5E, st["fixh"])
        m.w32(sp + 0x62, st["fixv"])
        m.w16(sp + 0x4E, st["speedv"])
        m.w16(sp + 0x50, st["speedh"])
        m.w16(sp + 0x8E, st["air"])
        m.w16(sp + 0x94, st["f94"])
        m.write(hot, struct.pack(">hhhh", *hr))
        snd.clear()
        m.call("HandleRampCollision", sp, half, hot, ramp, 209)
        o = {"pv": m.r16(sp + 2), "ph": m.r16(sp + 4), "fixh": struct.unpack(">i", m.read(sp + 0x5E, 4))[0],
             "fixv": struct.unpack(">i", m.read(sp + 0x62, 4))[0], "speedv": m.r16(sp + 0x4E),
             "speedh": m.r16(sp + 0x50), "air": m.r16(sp + 0x8E), "f94": m.r16(sp + 0x94), "sound": len(snd)}
        out.write(f"{fmt(st)} -> {dump(m, ramp, faces, tv)} | {fmt(o)}\n")
    out.close()
    print(f"rampcollide: {COLLIDES} cases")

    out = open(f"{work}/diff/ramptask.txt", "w")
    out.write("# ramp task: name=value inputs -> dump\n")
    for _ in range(TASKS):
        m.write(lvl + 0x10000, data[0x10000:0x11000])
        kind = rnd.choice(KINDS + [9700, 9705])
        st = {"kind": kind, "v": rnd.randint(100, 3800), "h": rnd.randint(100, 3800),
              "levelindex": rnd.choice([-1, rnd.randint(0, 255)]), "info1": rnd.choice([0, 1, 1, -1]),
              "speedv": rnd.choice([0, rnd.randint(-30, 30), rnd.randint(-1500, 1500)]),
              "speedh": rnd.choice([0, rnd.randint(-30, 30), rnd.randint(-1500, 1500)]),
              "air": rnd.choice([0, 0, 0, 30, 300]), "frames": rnd.randint(0, 20), "frozen": int(rnd.random() < 0.05)}
        if st["levelindex"] >= 0:
            m.w16(lvl + 0x10006 + st["levelindex"] * 16, st["info1"])
        m.write(sp, bytes(0xA2))
        m.w16(sp, kind)
        m.w16(sp + 2, st["v"])
        m.w16(sp + 4, st["h"])
        m.call("SetupRampSprite", sp)
        m.w16(sp + 0x68, st["levelindex"])
        m.w16(sp + 0x4E, st["speedv"])
        m.w16(sp + 0x50, st["speedh"])
        m.w16(sp + 0x8E, st["air"])
        m.write(sp + 0x92, bytes([st["frozen"]]))
        for _f in range(st["frames"]):
            m.call("HandleRampSprite", sp)
        out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)}\n")
    out.close()
    print(f"ramptask: {TASKS} cases")


if __name__ == "__main__":
    main()
