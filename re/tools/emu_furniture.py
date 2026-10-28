"""Record barrels (550) and chairs (560).

furnituretask.txt: SetupBarrelSprite / SetupChairSprite, then frames of
the task (burning, the explosion, the spin, HandleGlide, HandleHalfPipe,
JumpScale - r30 is the record, as SATRun leaves it) each followed by
CheckTileCollisions with the object as the only sprite (WallBounce /
BGTileBounce with its mask) - on the real levels, weighted to obstacles,
wall edges and the special floors.

furniturehit.txt: HitBarrelSprite / HitChairSprite with Harry (kicking or
walking into it), a barrel, a chair, a plant, a cooler, a door, a
collider, a ramp, stairs, a sliding door, player / enemy shots and explosions or flames (made
as explosions with a flame's kind and f7c). Every
sprite is made by its real setup.

Each line: "<name=value ...> -> <dump> [|| <other dump>] | <outputs>"
  events  p:<snd>, 3d:<snd>, 3d:<snd>:<rate> (pitched),
          x:<h>:<v>:<speed h>:<speed v>:<kind>:<dir> (EasyExplosion)
Usage: python3 -I emu_furniture.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from sprites import Faces, tvectors, dump  # noqa: E402
from emu_mask import masks  # noqa: E402
from emu_air import cells  # noqa: E402
from cicn import decode as cicn_decode  # noqa: E402

SAT, WALLS, OBSTACLE, PHYSICS, CONTACT = 0x10065B70, 0x100658D4, 0x100658D0, 0x100659D8, 0x10065E1C
BODY_FACE, PLANT_MASK, CHAIR_MASK, BARREL_MASK = 0x10065EA0, 0x10065ED0, 0x10065ECC, 0x10065EF0
SHAKE, CAFFEINE = 0x10065AD0, 0x10065AD4
SOUNDS = {0x10065BD8: 801, 0x10065BDC: 800, 0x10065C2C: 258, 0x10065C30: 257, 0x10065BB8: 1101, 0x10065BBC: 1100}
DOORS = [901, 902, 903, 911, 912, 913, 921, 922, 923, 931, 932, 933]
TASKS_PER_LEVEL = 800
HITS = 40000


def s16(x):
    x &= 0xFFFF
    return x - 0x10000 if x & 0x8000 else x


def cicn_mask(work, n):
    w, h, mk, _, _ = cicn_decode(open(f"{work}/res/Harry_Graphics/cicn_{n}.bin", "rb").read())
    return {(x, y) for y in range(h) for x in range(w) if mk[y][x]}


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m)
    m.stub_routine(0x1005F340, lambda mm: mm.alloc(0x40))      # the mask loader; filled below
    for f in ("InitBarrelSprite", "InitChairSprite", "InitPlantSprite", "InitCoolerSprite", "InitDoorSprite",
              "InitRampSprite", "InitStairsSprite", "InitSlidingDoorSprite", "InitEffectSprite", "InitTileHotRects", "InitPlayerKickRects"):
        m.call(f)
    tv = tvectors(f"{work}/ppc")
    by_name = {v: k for k, v in tv.items()}
    ms = masks(work)
    plant_mask, barrel_mask = cicn_mask(work, 599), cicn_mask(work, 598)
    ev = []
    m.stub_routine("STPlayRegSound", lambda mm: ev.append("p:%d" % mm.get(3)) or 0)
    m.stub_routine("STPlay3DSound", lambda mm: ev.append("3d:%d" % mm.get(3)) or 0)
    m.stub_routine("STPlay3DSoundPitched", lambda mm: ev.append("3d:%d:%d" % (mm.get(3), mm.get(7))) or 0)
    m.stub_routine("EasyExplosion", lambda mm: ev.append("x:%d:%d:%d:%d:%d:%d" % tuple(s16(mm.get(i)) for i in (3, 4, 5, 6, 7, 8))) or 0)
    for f in ("GammaFadeOut", "GammaFadeIn", 0x1002549C, "AddFGRedraw"):
        m.stub_routine(f, 0)
    busy = [0]
    m.stub_routine(0x10056DE8, lambda mm: busy[0])
    for slot, snd in SOUNDS.items():
        m.w32(m.glob(slot), snd)
    sat = m.glob(SAT)
    walls = m.alloc(64 * 0x2E)
    m.w32(m.glob(WALLS), walls)
    obstacle = m.alloc(0x40)
    m.w32(m.glob(OBSTACLE), obstacle)
    body_face = m.alloc(0x40)
    m.w32(m.glob(BODY_FACE), body_face)
    mask_faces = [(m.r32(m.glob(PLANT_MASK)), plant_mask), (m.r32(m.glob(CHAIR_MASK)), plant_mask),
                  (m.r32(m.glob(BARREL_MASK)), barrel_mask)]
    physics, contact, shake, caffeine = m.glob(PHYSICS), m.glob(CONTACT), m.glob(SHAKE), m.glob(CAFFEINE)
    for i, v in enumerate((50, 70, 245, 252, 150)):
        m.w16(physics + 2 * i, v)

    def regions():
        m.regions.clear()
        for k in range(32):
            m.w32(walls + 0x1A + k * 0x2E, m.make_region(ms[f"wall{k}"]))
        m.w32(obstacle + 0x1A, m.make_region(ms["obstacle"]))
        m.w32(body_face + 0x1A, m.make_region(ms["body"]))
        for f, mk in mask_faces:
            m.w32(f + 0x1A, m.make_region(mk))

    fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())
    sp = m.alloc(0xA2)
    rnd = random.Random("furniture")

    # --- the tasks ---
    out = open(f"{work}/diff/furnituretask.txt", "w")
    out.write("# barrel / chair task: name=value inputs -> dump | entry shake events\n")
    n = 0
    for lvl in range(1, 16):
        data = open(f"{work}/res/Episode_1/Hlvl_{lvl}.bin", "rb").read()
        base = m.load_level(data)
        mark = m.heap
        special = cells(data, set(range(1, 16)))
        grid = lambda x, y: struct.unpack(">H", data[0x8000 + (x * 128 + y) * 2:0x8000 + (x * 128 + y) * 2 + 2])[0]
        edges = [(x, y) for x in range(1, 127) for y in range(1, 127) if grid(x, y) == 0xFFFF and any(
            grid(a, b) != 0xFFFF for a, b in ((x, y - 1), (x + 1, y), (x - 1, y), (x, y + 1)))]
        for _ in range(TASKS_PER_LEVEL):
            m.heap = mark
            regions()
            m.write(base + 0x10000, data[0x10000:0x11000])
            kind = rnd.choice([550, 560])
            half = 12 if kind == 550 else 18
            pool = rnd.choice([special, edges, edges]) or edges
            x, y = rnd.choice(pool) if pool else (rnd.randint(2, 125), rnd.randint(2, 125))
            st = {"level": lvl, "kind": kind, "rec": sp, "h": x * 32 - half + rnd.randint(-20, 20),
                  "v": y * 32 - half + rnd.randint(-20, 20),
                  "speedh": rnd.choice([0, rnd.randint(-30, 30), rnd.randint(-2500, 2500)]),
                  "speedv": rnd.choice([0, rnd.randint(-30, 30), rnd.randint(-2500, 2500)]),
                  "air": rnd.choice([0, 0, 0, 0, 1, 20, 30, 216, 999, 1000, 1029, 1030, 1044]),
                  "slide": rnd.choice([0, 0, rnd.randint(0, 15), 87, 88, 100]), "frames": rnd.randint(1, 8),
                  "index": rnd.choice([-1, rnd.randint(0, 255), rnd.randint(0, 20)]), "seed": rnd.randint(1, 0x7FFFFFFE)}
            if kind == 550:
                st["dir"] = rnd.choice([1, 1, rnd.randint(0, 32), rnd.randint(15, 22)])
                st["timer"] = rnd.choice([0, 0, rnd.randint(-2, 30)])
            else:
                st["timer"] = rnd.choice([0, rnd.randint(-300, 300), rnd.randint(-5000, 5000)])
            m.write(sp, bytes(0xA2))
            m.w16(sp, kind)
            m.w16(sp + 2, st["v"])
            m.w16(sp + 4, st["h"])
            m.rand_seed = st["seed"]
            m.call("SetupBarrelSprite" if kind == 550 else "SetupChairSprite", sp)
            for off, k in ((0x50, "speedh"), (0x4E, "speedv"), (0x8E, "air"), (0x9C, "slide"), (0x74, "timer"), (0x66, "dir")):
                if k in st:
                    m.w16(sp + off, st[k])
            m.w16(sp + 0x68, st["index"])
            m.w16(shake, 0)
            pface = m.alloc(0x1C)
            m.write(pface, struct.pack(">hhhh", 0, 0, 2 * half, 2 * half))
            ev.clear()
            for _f in range(st["frames"]):
                m.call("HandleBarrelSprite" if kind == 550 else "HandleChairSprite", sp, regs={30: sp})
                real = m.r32(sp + 0x7E)
                m.w32(sp + 0x7E, pface)
                m.w32(sat + 0x78, sp)
                m.w32(sp + 0x2E, 0)
                m.call("CheckTileCollisions")
                m.w32(sp + 0x7E, real)
            e = m.read(base + 0x10000 + st["index"] * 16, 1)[0] if 0 <= st["index"] < 256 else -1
            out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} | entry={e} shake={m.r16(shake)} events={','.join(ev) or '-'}\n")
            n += 1
    out.close()
    print(f"furnituretask: {n} cases")

    # --- the hit tasks ---
    data = open(f"{work}/res/Episode_1/Hlvl_1.bin", "rb").read()
    base = m.load_level(data)
    other = m.alloc(0xA2)
    mark = m.heap
    out = open(f"{work}/diff/furniturehit.txt", "w")
    out.write("# HitBarrelSprite / HitChairSprite: name=value inputs -> dump || other dump | contact events\n")
    # other: 1 Harry, 2 barrel, 3 chair, 4 plant, 5 cooler, 6 door, 7 collider, 8 ramp, 9 player shot,
    # 10 enemy shot, 11 explosion, 12 stairs, 13 sliding door
    for _ in range(HITS):
        m.heap = mark
        regions()
        kind = rnd.choice([550, 560])
        v, h = rnd.randint(200, 3800), rnd.randint(200, 3800)
        ok = rnd.choice([1, 1, 1, 1, 2, 2, 3, 3, 4, 5, 6, 7, 8, 9, 10, 11, 11, 12, 13])
        st = {"kind": kind, "v": v, "h": h, "other": ok, "okind": 0,
              "ov": v + rnd.randint(-40, 40), "oh": h + rnd.randint(-40, 40), "samefix": int(rnd.random() < 0.05),
              "speedh": rnd.randint(-2500, 2500), "speedv": rnd.randint(-2500, 2500),
              "ospeedh": rnd.randint(-2500, 2500), "ospeedv": rnd.randint(-2500, 2500),
              "air": rnd.choice([0, 0, 0, 0, 5]), "oair": rnd.choice([0, 0, 0, 0, 5]),
              "frozen": int(rnd.random() < 0.03), "ofrozen": int(rnd.random() < 0.03),
              "kicking": int(rnd.random() < 0.4), "odir": rnd.randint(0, 35), "busy": rnd.choice([0, 0, 1]),
              "contact": rnd.choice([0, 64]), "caffeine": rnd.choice([0, 0, 5]),
              "dir": rnd.choice([1, 1, 1, 0, 2, 5]), "timer": rnd.choice([0, 0, 10, 17, 18, 20, rnd.randint(-500, 500)]),
              "blast": rnd.choice([0, 0, 0, 1, 5, -1]), "f94": rnd.choice([0, 0, 0, 2]), "seed": rnd.randint(1, 0x7FFFFFFE)}
        if ok == 1 and rnd.random() < 0.6:
            # Harry next to it, where his kick reaches
            st["ov"], st["oh"] = v - 20 + rnd.randint(-28, 28), h - 20 + rnd.randint(-28, 28)
            st["kicking"] = int(rnd.random() < 0.7)
        if ok == 6:
            st["okind"] = rnd.choice(DOORS)
        elif ok == 7:
            st["okind"] = rnd.choice([3100, 9200, 9600, 2301])
        elif ok == 8:
            st["okind"] = rnd.choice([9701, 9702, 9703, 9704])
        elif ok == 12:
            st["okind"] = rnd.choice([9901, 9902, 9903, 9904])
        elif ok == 13:
            st["okind"] = rnd.choice([951, 961])
        elif ok in (9, 10):
            st["okind"] = rnd.choice([0, 1, 9450, 9610])
        elif ok == 11:
            st["okind"] = rnd.choice([0, 14, 12, -1, 4200, 4203])
        m.rand_seed = st["seed"]
        m.write(sp, bytes(0xA2))
        m.w16(sp, kind)
        m.w16(sp + 2, v)
        m.w16(sp + 4, h)
        m.call("SetupBarrelSprite" if kind == 550 else "SetupChairSprite", sp)
        m.w16(sp + 0x5A, v)
        m.w16(sp + 0x5C, h)
        if rnd.random() < 0.5:
            m.w32(sp + 0x5E, (h << 8) + rnd.randint(0, 255))
            m.w32(sp + 0x62, (v << 8) + rnd.randint(0, 255))
        m.write(other, bytes(0xA2))
        m.w16(other, {2: 550, 3: 560, 4: 601, 5: 500}.get(ok, st["okind"]))
        m.w16(other + 2, st["ov"])
        m.w16(other + 4, st["oh"])
        setup = {2: "SetupBarrelSprite", 3: "SetupChairSprite", 4: "SetupPlantSprite", 5: "SetupCoolerSprite",
                 6: "SetupDoorSprite", 8: "SetupRampSprite", 11: "SetupEffectSprite", 12: "SetupStairsSprite",
                 13: "SetupSlidingDoorSprite"}.get(ok)
        if setup:
            if ok == 11:
                # the flames' setup comes with the fire pipes: an explosion's,
                # then kind and f7c of the flame
                m.w16(other, 0)
            m.call(setup, other)
            m.w16(other + 0x5A, st["ov"])
            m.w16(other + 0x5C, st["oh"])
            if ok == 11:
                m.w16(other, st["okind"])
                m.w16(other + 0x7C, st["okind"])
        else:
            task = {1: "HandlePlayerSprite", 7: "HandleColliderSprite", 9: "HandlePlayerShotSprite",
                    10: "HandleEnemyShotSprite"}[ok]
            m.w32(other + 0x1A, by_name[task])
            m.w16(other + 0x5A, st["ov"])
            m.w16(other + 0x5C, st["oh"])
            m.w32(other + 0x5E, (st["oh"] << 8) + rnd.randint(0, 255))
            m.w32(other + 0x62, (st["ov"] << 8) + rnd.randint(0, 255))
            if ok == 1:
                m.w16(other, 69)
                m.write(other + 6, struct.pack(">hhhh", *((0, 0, 64, 64) if st["kicking"] else (22, 22, 42, 42))))
                m.w16(other + 2, st["ov"] + rnd.choice([0, 0, rnd.randint(-3, 3)]))
                m.w16(other + 4, st["oh"] + rnd.choice([0, 0, rnd.randint(-3, 3)]))
            elif ok == 7:
                m.write(other + 6, struct.pack(">hhhh", 0, 0, rnd.randint(16, 48), rnd.randint(16, 48)))
        if st["samefix"]:
            m.w32(other + 0x5E, struct.unpack(">i", m.read(sp + 0x5E, 4))[0] + rnd.randint(0, 255))
            m.w32(other + 0x62, struct.unpack(">i", m.read(sp + 0x62, 4))[0] + rnd.randint(0, 255))
        for s_, pre in ((sp, ""), (other, "o")):
            m.w16(s_ + 0x50, st[pre + "speedh"])
            m.w16(s_ + 0x4E, st[pre + "speedv"])
            m.w16(s_ + 0x8E, st[pre + "air"])
            m.write(s_ + 0x92, bytes([st[pre + "frozen"]]))
        if ok == 11:
            st["odir"] = rnd.randint(0, 26)
        m.w16(other + 0x66, st["odir"])
        m.w16(sp + 0x66, st["dir"])
        m.w16(sp + 0x74, st["timer"])
        m.w16(sp + 0x96, st["blast"])
        m.w16(sp + 0x94, st["f94"])
        for s_, pre in ((sp, ""), (other, "o")):
            st[pre + "pv"], st[pre + "ph"] = m.r16(s_ + 2), m.r16(s_ + 4)
            st[pre + "fixh"] = struct.unpack(">i", m.read(s_ + 0x5E, 4))[0]
            st[pre + "fixv"] = struct.unpack(">i", m.read(s_ + 0x62, 4))[0]
            st[pre + "hot"] = ":".join(str(s16(m.r16(s_ + o))) for o in (6, 8, 10, 12))
            st[pre + "f6e"] = struct.unpack(">i", m.read(s_ + 0x6E, 4))[0]
        m.w16(contact, st["contact"])
        m.w16(caffeine, st["caffeine"])
        busy[0] = st["busy"]
        st["seed2"] = rnd.randint(1, 0x7FFFFFFE)
        m.rand_seed = st["seed2"]
        ev.clear()
        m.call("HitBarrelSprite" if kind == 550 else "HitChairSprite", sp, other)
        out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} || {dump(m, other, faces, tv)} | "
                  f"contact={m.r16(contact)} events={','.join(ev) or '-'}\n")
    out.close()
    print(f"furniturehit: {HITS} cases")


if __name__ == "__main__":
    main()
