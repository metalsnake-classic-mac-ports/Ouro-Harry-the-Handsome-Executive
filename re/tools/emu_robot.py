"""Record robots (1801-1821, 2001-2021).

robottask.txt: SetupRobotSprite at the robots' places in Episode 1 (and
nearby) with Harry (gPlayerH/V) somewhere around, then frames of
HandleRobotSprite each followed by CheckTileCollisions with the robot as
the only sprite; Harry moves a little between frames (steps). Hit points, modes
and timers start at random. SATNewSprite is stubbed (a zeroed record with
pos and fix as SetupEnemyShotSprite leaves them), GenerateSprite logged.

robothit.txt: HitRobotSprite with Harry (kicking or walking), a barrel,
an explosion, a ramp, stairs, plants, coolers, colliders, co-workers,
chairs, doors and sliding doors.

Each line: "<name=value ...> -> <dump> [|| <other dump>] | <outputs>"
  events  per frame ';'-joined: p:/3d:<snd>, 3d:<snd>:<rate> (pitched),
          x:<h>:<v>:<speed h>:<speed v>:<kind>:<dir> (EasyExplosion),
          shot:<kind>:<h>:<v>:<speed v>:<speed h>:<dir>:<fix h>:<fix v>,
          gen:<kind>:<h>:<v> (GenerateSprite)
Usage: python3 -I emu_robot.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from sprites import Faces, tvectors, dump, face_region  # noqa: E402
from emu_mask import masks  # noqa: E402
from cicn import decode as cicn_decode  # noqa: E402

SAT, LEVEL_H, WALLS, OBSTACLE, PHYSICS, CONTACT = 0x10065B70, 0x10065B64, 0x100658D4, 0x100658D0, 0x100659D8, 0x10065E1C
PLAYER_H, PLAYER_V, SCORE, CHATTER, CAFFEINE = 0x100659B4, 0x100659B0, 0x10065AF8, 0x10065AC8, 0x10065AD4
ROBOT_MASK = 0x10066180
FLAGS = (0x1006619C, 0x10066198)                # a robot bounced off a wall / an obstacle
SOUNDS = {0x10065BFC: 600, 0x10065BF8: 601, 0x10065BF4: 602, 0x10065C1C: 606, 0x10065BD8: 801}
KINDS = [1801, 1811, 1821, 2001, 2011, 2021]
RUNS = 8000
HITS = 30000
DOORS = [901, 902, 903, 911, 912, 913, 921, 922, 923, 931, 932, 933]


def s16(x):
    x &= 0xFFFF
    return x - 0x10000 if x & 0x8000 else x


def cicn_mask(work, n):
    w, h, mk, _, _ = cicn_decode(open(f"{work}/res/Harry_Graphics/cicn_{n}.bin", "rb").read())
    return {(x, y) for y in range(h) for x in range(w) if mk[y][x]}


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m, work)
    m.stub_routine(0x1005F340, lambda mm: mm.alloc(0x40))
    for f in ("InitRobotSprite", "InitTileHotRects", "InitPlayerKickRects", "InitBarrelSprite", "InitPlantSprite",
              "InitCoolerSprite", "InitDoorSprite", "InitRampSprite", "InitStairsSprite", "InitSlidingDoorSprite",
              "InitCoWorkerSprite", "InitChairSprite", "InitColliderSprite", "InitEffectSprite", "InitGrateSprite"):
        m.call(f)
    tv = tvectors(f"{work}/ppc")
    by_name = {v: k for k, v in tv.items()}
    ms = masks(work)
    ev, news = [], []
    m.stub_routine("STPlayRegSound", lambda mm: ev.append("p:%d" % mm.get(3)) or 0)
    m.stub_routine("STPlay3DSound", lambda mm: ev.append("3d:%d" % mm.get(3)) or 0)
    m.stub_routine("STPlay3DSoundPitched", lambda mm: ev.append("3d:%d:%d" % (mm.get(3), mm.get(7))) or 0)
    m.stub_routine("EasyExplosion", lambda mm: ev.append("x:%d:%d:%d:%d:%d:%d" % tuple(s16(mm.get(i)) for i in (3, 4, 5, 6, 7, 8))) or 0)
    m.stub_routine("GenerateSprite", lambda mm: ev.append("gen:%d:%d:%d" % (s16(mm.get(5)), s16(mm.get(3)), s16(mm.get(4)))) or 0)
    m.stub_routine(0x10056DE8, 0)
    for f in ("GammaFadeOut", "GammaFadeIn", 0x1002549C, "AddFGRedraw"):
        m.stub_routine(f, 0)

    def new_sprite(mm):
        rec = mm.alloc(0xA2)
        kind, h, v = s16(mm.get(3)), s16(mm.get(4)), s16(mm.get(5))
        mm.w16(rec, kind)
        mm.w16(rec + 2, v)
        mm.w16(rec + 4, h)
        mm.w32(rec + 0x5E, (h << 8) & 0xFFFFFFFF)
        mm.w32(rec + 0x62, (v << 8) & 0xFFFFFFFF)
        news.append((rec, kind, h, v))
        ev.append(rec)
        return rec
    m.stub_routine("SATNewSprite", new_sprite)

    for slot, snd in SOUNDS.items():
        m.w32(m.glob(slot), snd)
    sat = m.glob(SAT)
    walls = m.alloc(64 * 0x2E)
    m.w32(m.glob(WALLS), walls)
    obstacle = m.alloc(0x40)
    m.w32(m.glob(OBSTACLE), obstacle)
    robot_face = m.r32(m.glob(ROBOT_MASK))
    robot_mask = face_region(work, 4251)            # cicn 598 with the faces' mirrored bits
    physics = m.glob(PHYSICS)
    for i, v in enumerate((50, 70, 245, 252, 150)):
        m.w16(physics + 2 * i, v)

    def regions():
        m.regions.clear()
        for k in range(32):
            m.w32(walls + 0x1A + k * 0x2E, m.make_region(ms[f"wall{k}"]))
        m.w32(obstacle + 0x1A, m.make_region(ms["obstacle"]))
        m.w32(robot_face + 0x14, m.make_region(robot_mask))

    def frame_events():
        out = []
        made = {n[0]: n for n in news}
        for e in ev:
            if isinstance(e, int):
                _, kind, h, v = made[e]
                out.append("shot:%d:%d:%d:%d:%d:%d:%d:%d" % (
                    kind, h, v, s16(m.r16(e + 0x4E)), s16(m.r16(e + 0x50)), s16(m.r16(e + 0x66)),
                    struct.unpack(">i", m.read(e + 0x5E, 4))[0], struct.unpack(">i", m.read(e + 0x62, 4))[0]))
            else:
                out.append(e)
        return ",".join(out) or "-"

    fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())
    sp = m.alloc(0xA2)
    mark = m.heap
    rnd = random.Random("robot")
    places = []
    for lvl in range(1, 16):
        data = open(f"{work}/res/Episode_1/Hlvl_{lvl}.bin", "rb").read()
        for i in range(256):
            e = data[0x10000 + i * 16:0x10000 + i * 16 + 16]
            kind = struct.unpack(">h", e[2:4])[0]
            if e[0] and (1800 <= kind <= 1849 or 2000 <= kind <= 2099):
                v, h = struct.unpack(">hh", e[12:16])
                places.append((lvl, i, kind, v - 6, h - 6))
    levels = {}

    def use_level(lvl):
        if lvl not in levels:
            data = open(f"{work}/res/Episode_1/Hlvl_{lvl}.bin", "rb").read()
            m.load_level(data)
            levels[lvl] = (data, m.r32(m.glob(LEVEL_H)))
        data, handle = levels[lvl]
        if m.r32(m.glob(LEVEL_H)) != handle:
            m.w32(m.glob(LEVEL_H), handle)
            m.call("LoadTileDefinitions")
        return data, m.r32(handle)

    out = open(f"{work}/diff/robottask.txt", "w")
    out.write("# robot task: name=value inputs -> dump | entry score chatter events\n")
    for _ in range(RUNS):
        lvl, index, kind, v, h = rnd.choice(places)
        data, base = use_level(lvl)
        mark = m.heap
        regions()
        m.write(base + 0x10000, data[0x10000:0x11000])
        st = {"level": lvl, "index": index, "kind": rnd.choice([kind, kind, rnd.choice(KINDS)]),
              "v": v + rnd.choice([0, 0, rnd.randint(-100, 100)]), "h": h + rnd.choice([0, 0, rnd.randint(-100, 100)]),
              "pv": v + rnd.choice([rnd.randint(-300, 300), rnd.randint(-700, 700), rnd.randint(-60, 60)]),
              "ph": h + rnd.choice([rnd.randint(-300, 300), rnd.randint(-700, 700), rnd.randint(-60, 60)]),
              "hp": rnd.choice([-1, 0, 1, 3, 3, 8]), "mode": rnd.choice([-1, -1, -1, 0, 1, 2, 3, 4, 5, 6, 7]),
              "timer": rnd.choice([0, 0, 3, 25]), "speedh": rnd.choice([0, 0, rnd.randint(-600, 600)]),
              "speedv": rnd.choice([0, 0, rnd.randint(-600, 600)]), "air": rnd.choice([0, 0, 0, 0, 5, 30]),
              "info3": rnd.choice([0, 0, 0, 702, 11399]), "frozen": int(rnd.random() < 0.02),
              "frames": rnd.randint(1, 80), "seed": rnd.randint(1, 0x7FFFFFFE), "walk": rnd.choice([0, 0, 1])}
        st["steps"] = ",".join(f"{rnd.randint(-6, 6)}:{rnd.randint(-6, 6)}" if st["walk"] else "0:0"
                               for _f in range(st["frames"]))
        e = base + 0x10000 + index * 16
        m.w16(e + 10, st["info3"])
        m.w16(m.glob(PLAYER_H), st["ph"])
        m.w16(m.glob(PLAYER_V), st["pv"])
        m.w32(m.glob(SCORE), 0)
        m.w16(m.glob(CHATTER), 0)
        for flag in FLAGS:
            m.write(m.glob(flag), b"\0")
        m.write(sp, bytes(0xA2))
        m.w16(sp, st["kind"])
        m.w16(sp + 2, st["v"])
        m.w16(sp + 4, st["h"])
        m.call("SetupRobotSprite", sp)
        m.w16(sp + 0x68, index)
        if st["hp"] != -1:
            m.w16(sp + 0x72, st["hp"])
        if st["mode"] != -1:
            m.w16(sp + 0x7C, st["mode"])
        m.w16(sp + 0x74, st["timer"])
        m.w16(sp + 0x50, st["speedh"])
        m.w16(sp + 0x4E, st["speedv"])
        m.w16(sp + 0x8E, st["air"])
        m.write(sp + 0x92, bytes([st["frozen"]]))
        m.rand_seed = st["seed"]
        evs = []
        for step in st["steps"].split(","):
            dh, dv = map(int, step.split(":"))
            m.w16(m.glob(PLAYER_H), m.r16(m.glob(PLAYER_H)) + dh)
            m.w16(m.glob(PLAYER_V), m.r16(m.glob(PLAYER_V)) + dv)
            ev.clear()
            news.clear()
            m.call("HandleRobotSprite", sp, regs={30: sp})
            m.w32(sat + 0x78, sp)
            m.w32(sp + 0x2E, 0)
            m.call("CheckTileCollisions")
            evs.append(frame_events())
        o = f"entry={m.read(e, 1)[0]} score={m.r32(m.glob(SCORE))} chatter={s16(m.r16(m.glob(CHATTER)))} events={';'.join(evs)}"
        out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} | {o}\n")
        m.heap = mark
    out.close()
    print(f"robottask: {RUNS} cases")

    data, base = use_level(4)
    other = m.alloc(0xA2)
    mark = m.heap
    out = open(f"{work}/diff/robothit.txt", "w")
    out.write("# HitRobotSprite: name=value inputs -> dump || other dump | contact events\n")
    partners = ["harry", "harry", "barrel", "effect", "ramp", "stairs", "plant", "cooler", "collider", "coworker",
                "chair", "door", "sliding", "grate"]
    setups = {"barrel": ("SetupBarrelSprite", [550]), "effect": ("SetupEffectSprite", [0, 14, -1]),
              "ramp": ("SetupRampSprite", [9701, 9702, 9703, 9704]), "stairs": ("SetupStairsSprite", [9901, 9902, 9903, 9904]),
              "plant": ("SetupPlantSprite", [601]), "cooler": ("SetupCoolerSprite", [500]),
              "collider": ("SetupColliderSprite", [9200, 9600, 3100]), "coworker": ("SetupCoWorkerSprite", [1301, 1321]),
              "chair": ("SetupChairSprite", [560]), "door": ("SetupDoorSprite", DOORS), "sliding": ("SetupSlidingDoorSprite", [951, 961]),
              "grate": ("SetupGrateSprite", [3900, 3905, 3910, 3915])}
    for _ in range(HITS):
        m.heap = mark
        m.regions.clear()
        p = rnd.choice(partners)
        v, h = rnd.randint(300, 3700), rnd.randint(300, 3700)
        st = {"kind": rnd.choice(KINDS), "v": v, "h": h, "other": p, "okind": 69,
              "ov": v + rnd.randint(-40, 40), "oh": h + rnd.randint(-40, 40), "harryv": v - 200, "harryh": h - 200,
              "speedh": rnd.randint(-900, 900), "speedv": rnd.randint(-900, 900),
              "ospeedh": rnd.randint(-900, 900), "ospeedv": rnd.randint(-900, 900),
              "mode": rnd.choice([0, 1, 2, 2, 4]), "timer": rnd.choice([0, 0, 3]), "odir": rnd.randint(0, 35),
              "kicking": int(rnd.random() < 0.5), "caffeine": rnd.choice([0, 0, 5]), "contact": rnd.choice([0, 64]),
              "frozen": int(rnd.random() < 0.03), "ofrozen": int(rnd.random() < 0.03), "blast": rnd.choice([0, 0, 2]),
              "f94": rnd.choice([0, 0, 2]), "seed": rnd.randint(1, 0x7FFFFFFE), "seed2": rnd.randint(1, 0x7FFFFFFE)}
        if p == "harry" and rnd.random() < 0.6:
            st["ov"], st["oh"] = v - 20 + rnd.randint(-24, 24), h - 20 + rnd.randint(-24, 24)
        if p != "harry":
            st["okind"] = rnd.choice(setups[p][1])
        m.w16(m.glob(PLAYER_H), st["harryh"])
        m.w16(m.glob(PLAYER_V), st["harryv"])
        m.rand_seed = st["seed"]
        m.write(sp, bytes(0xA2))
        m.w16(sp, st["kind"])
        m.w16(sp + 2, v)
        m.w16(sp + 4, h)
        m.call("SetupRobotSprite", sp)
        m.write(other, bytes(0xA2))
        m.w16(other, st["okind"] if p != "effect" else 0)
        m.w16(other + 2, st["ov"])
        m.w16(other + 4, st["oh"])
        if p == "harry":
            m.w32(other + 0x1A, by_name["HandlePlayerSprite"])
            m.write(other + 6, struct.pack(">hhhh", *((0, 0, 64, 64) if st["kicking"] else (22, 22, 42, 42))))
        else:
            m.call(setups[p][0], other)
            if p == "effect":
                m.w16(other, st["okind"])
        m.w16(other + 0x5A, st["ov"])
        m.w16(other + 0x5C, st["oh"])
        m.w16(other + 0x66, st["odir"] if p in ("harry", "barrel") else m.r16(other + 0x66))
        st["odir"] = s16(m.r16(other + 0x66))
        for s_, pre in ((sp, ""), (other, "o")):
            m.w16(s_ + 0x50, st[pre + "speedh"])
            m.w16(s_ + 0x4E, st[pre + "speedv"])
            m.write(s_ + 0x92, bytes([st[pre + "frozen"]]))
        m.w16(sp + 0x7C, st["mode"])
        m.w16(sp + 0x74, st["timer"])
        m.w16(sp + 0x96, st["blast"])
        m.w16(sp + 0x94, st["f94"])
        for s_, pre in ((sp, ""), (other, "o")):
            st[pre + "pv"], st[pre + "ph"] = s16(m.r16(s_ + 2)), s16(m.r16(s_ + 4))
            st[pre + "fixh"] = struct.unpack(">i", m.read(s_ + 0x5E, 4))[0]
            st[pre + "fixv"] = struct.unpack(">i", m.read(s_ + 0x62, 4))[0]
            st[pre + "hot"] = ":".join(str(s16(m.r16(s_ + o))) for o in (6, 8, 10, 12))
        m.w16(m.glob(CONTACT), st["contact"])
        m.w16(m.glob(CAFFEINE), st["caffeine"])
        m.rand_seed = st["seed2"]
        ev.clear()
        m.call("HitRobotSprite", sp, other)
        out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} || {dump(m, other, faces, tv)} | "
                  f"contact={s16(m.r16(m.glob(CONTACT)))} events={','.join(map(str, ev)) or '-'}\n")
    out.close()
    print(f"robothit: {HITS} cases")


if __name__ == "__main__":
    main()
