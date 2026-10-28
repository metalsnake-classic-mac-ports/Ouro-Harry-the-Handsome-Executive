"""Record the enemy shots.

flight: SetupEnemyShotSprite, then frames of HandleEnemyShotSprite each
followed by CheckTileCollisions with the shot as the only sprite
(HitEnemyShotTileSprite: WallBounce / BGTileBounce with the shots' mask,
cicn 1299) on the real levels, weighted to wall edges and obstacles.
Harry (for the homing shots) is a sprite at (hv, hh) with his normal hot
rect.
  line: "<name=value ...> -> <dump> | events=<...>"

aim: FindDesiredDirectionGeneric(to, from) for points near and on the
slope limits.
  line: "tov=.. toh=.. fromv=.. fromh=.. -> <dir>"

hit: HitEnemyShotSprite with a collider, cooler, plant, another enemy
shot, a player shot or a bonus, each made by its real setup.
  line: "<name=value ...> -> <dump> || <other dump> | events=<...>"

events: 3d:<snd>, p:<snd>, n:<kind>:<h>:<v>:<speed v>:<speed h>
Usage: python3 -I emu_enemyshot.py [re/work]
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

SAT, WALLS, OBSTACLE, SHOT_MASK, PLAYER_SPRITE = 0x10065B70, 0x100658D4, 0x100658D0, 0x10066054, 0x10065980
SOUNDS = {0x10065BAC: 4701, 0x10065BA4: 4703, 0x10065C48: 251, 0x10065BFC: 600, 0x10065BA8: 4702}
KINDS = [1018, 1028, 1038, 1048, 1051, 1101, 1150, 1299, 1101, 1299, 1800, 1900, 1950, 2050, 2001, 2500, 2281,
         2291, 9450, 9610, 9610, 9610, 3000, 500]
TV = {"player": 0x10066FC4, "playershot": 0x100672FC, "bonus": 0x1006702C}
FLIGHT_PER_LEVEL, HITS, AIMS = 1000, 12000, 30000


def s16(v):
    return ((v & 0xFFFF) ^ 0x8000) - 0x8000


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m)
    m.stub_routine(0x1005F340, lambda mm: mm.alloc(0x40))      # the mask loader; filled below
    for f in ("InitEnemyShotSprite", "InitTileHotRects", "InitColliderSprite", "InitCoolerSprite",
              "InitPlantSprite", "InitBonusSprite", "InitBarrelSprite", "InitChairSprite"):
        m.call(f)
    tv = tvectors(f"{work}/ppc")
    ms = masks(work)
    w, h, mk, _, _ = cicn_decode(open(f"{work}/res/Harry_Graphics/cicn_1299.bin", "rb").read())
    shot_mask = {(x, y) for y in range(h) for x in range(w) if mk[y][x]}
    ev = []
    m.stub_routine("STPlayRegSound", lambda mm: ev.append("p:%d" % mm.get(3)) or 0)
    m.stub_routine("STPlay3DSound", lambda mm: ev.append("3d:%d" % mm.get(3)) or 0)
    for f in ("STPlay3DSoundPitched", "GammaFadeOut", "GammaFadeIn", 0x1002549C, "AddFGRedraw", "EasyExplosion"):
        m.stub_routine(f, 0)
    m.stub_routine(0x10056DE8, 0)

    def new_sprite(mm):
        rec = mm.alloc(0xA2)
        ev.append(("n", rec, s16(mm.get(3)), s16(mm.get(4)), s16(mm.get(5))))
        return rec
    m.stub_routine("SATNewSprite", new_sprite)
    for slot, snd in SOUNDS.items():
        m.w32(m.glob(slot), snd)
    sat = m.glob(SAT)
    walls = m.alloc(64 * 0x2E)
    m.w32(m.glob(WALLS), walls)
    obstacle = m.alloc(0x40)
    m.w32(m.glob(OBSTACLE), obstacle)
    shot_face = m.r32(m.glob(SHOT_MASK))
    harry = m.alloc(0xA2)
    m.w32(m.glob(PLAYER_SPRITE), harry)
    sp, other = m.alloc(0xA2), m.alloc(0xA2)
    rnd = random.Random("enemyshot")
    fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())

    def resolve():
        for j, e in enumerate(ev):
            if isinstance(e, tuple):
                _, rec, k, nh, nv = e
                ev[j] = f"n:{k}:{nh}:{nv}:{m.r16(rec + 0x4E)}:{m.r16(rec + 0x50)}"

    def setup(kind, v, hh, st):
        m.write(sp, bytes(0xA2))
        m.w16(sp, kind)
        m.w16(sp + 2, v)
        m.w16(sp + 4, hh)
        m.call("SetupEnemyShotSprite", sp)
        for off, k in ((0x4E, "speedv"), (0x50, "speedh"), (0x66, "dir"), (0x7C, "f7c")):
            if st[k] != -9999:
                m.w16(sp + off, st[k])
        for off, k in ((0x74, "timer"),):
            if st[k] != -9999:
                m.w16(sp + off, st[k])
        for off, k in ((0x78, "f78"), (0x6E, "f6e")):
            if st[k] != -9999:
                m.w32(sp + off, st[k] & 0xFFFFFFFF)

    def inputs(kind):
        st = {"kind": kind,
              "speedv": rnd.choice([-9999, -9999, 0, rnd.randint(-299, 299), rnd.randint(-2500, 2500)]),
              "speedh": rnd.choice([-9999, -9999, 0, rnd.randint(-299, 299), rnd.randint(-2500, 2500)]),
              "dir": rnd.choice([-9999, -9999, rnd.randint(-1, 16)]),
              "f7c": rnd.choice([-9999, -9999, 0, 1, 2, rnd.randint(0, 12)]),
              "timer": rnd.choice([-9999, -9999, 0, rnd.randint(-20, 50)]),
              "f78": rnd.choice([-9999, -9999, 0, 30, 35, rnd.randint(0, 40)]),
              "f6e": rnd.choice([-9999, -9999, 0, 99, 100, 101])}
        if 1101 <= kind <= 1299:
            # a homing shot is started with timer 0 (or is bursting)
            st["timer"] = rnd.choice([0, 0, 0, -16, rnd.randint(-16, -3)])
        return st

    out = open(f"{work}/diff/enemyshot.txt", "w")
    out.write("# enemy shots in flight: name=value inputs -> dump | events\n")
    n = 0
    for lvl in range(1, 16):
        data = open(f"{work}/res/Episode_1/Hlvl_{lvl}.bin", "rb").read()
        m.load_level(data)
        mark = m.heap
        special = cells(data, {1})
        grid = lambda x, y: struct.unpack(">H", data[0x8000 + (x * 128 + y) * 2:0x8000 + (x * 128 + y) * 2 + 2])[0]
        edges = [(x, y) for x in range(1, 127) for y in range(1, 127) if grid(x, y) == 0xFFFF and any(
            grid(a, b) != 0xFFFF for a, b in ((x, y - 1), (x + 1, y), (x - 1, y), (x, y + 1)))]
        for _ in range(FLIGHT_PER_LEVEL):
            m.heap = mark
            m.regions.clear()
            for k in range(32):
                m.w32(walls + 0x1A + k * 0x2E, m.make_region(ms[f"wall{k}"]))
            m.w32(obstacle + 0x1A, m.make_region(ms["obstacle"]))
            m.w32(shot_face + 0x1A, m.make_region(shot_mask))
            kind = rnd.choice(KINDS)
            pool = rnd.choice([special, edges, edges]) or edges
            x, y = rnd.choice(pool)
            st = {"level": lvl, "h": x * 32 + rnd.randint(-24, 24), "v": y * 32 + rnd.randint(-24, 24), **inputs(kind),
                  "frames": rnd.choice([1, 2, rnd.randint(1, 20), rnd.randint(1, 60)])}
            st["hv"], st["hh"] = st["v"] + rnd.randint(-200, 200), st["h"] + rnd.randint(-200, 200)
            setup(kind, st["v"], st["h"], st)
            m.write(harry, bytes(0xA2))
            m.write(harry + 6, struct.pack(">hhhh", 22, 22, 42, 42))
            m.w16(harry + 0x5A, st["hv"])
            m.w16(harry + 0x5C, st["hh"])
            pface = m.alloc(0x1C)
            m.write(pface, struct.pack(">hhhh", 0, 0, 8, 8))
            ev.clear()
            for _f in range(st["frames"]):
                m.call("HandleEnemyShotSprite", sp)
                resolve()
                real = m.r32(sp + 0x7E)
                m.w32(sp + 0x7E, pface)
                m.w32(sat + 0x78, sp)
                m.w32(sp + 0x2E, 0)
                m.call("CheckTileCollisions")
                m.w32(sp + 0x7E, real)
            out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} | events={','.join(ev) or '-'}\n")
            n += 1
    out.close()
    print(f"enemyshot: {n} cases")

    m.load_level(open(f"{work}/res/Episode_1/Hlvl_1.bin", "rb").read())
    mark = m.heap
    out = open(f"{work}/diff/enemyshothit.txt", "w")
    out.write("# HitEnemyShotSprite: name=value inputs -> dump || other dump | events\n")
    for _ in range(HITS):
        m.heap = mark
        kind = rnd.choice(KINDS)
        v, hh = rnd.randint(200, 3800), rnd.randint(200, 3800)
        st = {"v": v, "h": hh, **inputs(kind),
              "other": rnd.choice(["collider", "collider", "cooler", "plant", "enemyshot", "enemyshot", "playershot",
                                   "bonus", "barrel", "chair"]),
              "okind": 0, "ov": v + rnd.randint(-30, 30), "oh": hh + rnd.randint(-30, 30),
              "frozen": int(rnd.random() < 0.05), "ofrozen": int(rnd.random() < 0.05)}
        if st["other"] == "collider":
            st["okind"] = rnd.choice([9200, 9300, 9411, 9421, 9600, 2301, 3100])
        elif st["other"] == "cooler":
            st["okind"] = 500
        elif st["other"] == "plant":
            st["okind"] = 601
        elif st["other"] == "enemyshot":
            st["okind"] = rnd.choice(KINDS)
        elif st["other"] == "bonus":
            st["okind"] = 700
        elif st["other"] in ("barrel", "chair"):
            st["okind"] = 550 if st["other"] == "barrel" else 560
        setup(kind, v, hh, st)
        m.write(sp + 0x92, bytes([st["frozen"]]))
        m.write(other, bytes(0xA2))
        m.w16(other, st["okind"])
        m.w16(other + 2, st["ov"])
        m.w16(other + 4, st["oh"])
        if st["other"] == "playershot":
            m.w32(other + 0x1A, TV["playershot"])
            m.w16(other + 0x5A, st["ov"])
            m.w16(other + 0x5C, st["oh"])
            m.write(other + 6, struct.pack(">hhhh", 0, 0, 8, 8))
        else:
            m.rand_seed = 1
            m.call({"collider": "SetupColliderSprite", "cooler": "SetupCoolerSprite", "plant": "SetupPlantSprite",
                    "enemyshot": "SetupEnemyShotSprite", "bonus": "SetupBonusSprite", "barrel": "SetupBarrelSprite",
                    "chair": "SetupChairSprite"}[st["other"]], other)
        if st["other"] == "enemyshot" and 1101 <= st["okind"] <= 1299:
            m.w16(other + 0x74, 0)
        m.write(other + 0x92, bytes([st["ofrozen"]]))
        ev.clear()
        m.call("HitEnemyShotSprite", sp, other)
        out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} || {dump(m, other, faces, tv)} | events={','.join(ev) or '-'}\n")
    out.close()
    print(f"enemyshothit: {HITS} cases")

    limits = [0.0875, 0.2675, 0.2679, 0.4663, 0.7002, 1.0, 1.4281, 2.1445, 3.732, 11.43]
    out = open(f"{work}/diff/aim.txt", "w")
    out.write("# FindDesiredDirectionGeneric: to, from -> direction\n")
    for _ in range(AIMS):
        fv, fh = rnd.randint(0, 4000), rnd.randint(0, 4000)
        if rnd.random() < 0.7:
            dx = rnd.choice([1, 2, 3, rnd.randint(1, 50), rnd.randint(1, 3000)]) * rnd.choice([-1, 1])
            t = rnd.choice(limits) * rnd.choice([-1, 1])
            dy = int(round(t * dx)) + rnd.choice([-1, 0, 0, 1])
            # dx = from.h - to.h, dy = to.v - from.v
            th, tv = fh - dx, fv + dy
        else:
            th, tv = fh + rnd.choice([0, rnd.randint(-3, 3), rnd.randint(-3000, 3000)]), \
                fv + rnd.choice([0, rnd.randint(-3, 3), rnd.randint(-3000, 3000)])
        tv, th = s16(tv), s16(th)
        r = m.call("FindDesiredDirectionGeneric", ((tv & 0xFFFF) << 16) | (th & 0xFFFF),
                   ((fv & 0xFFFF) << 16) | (fh & 0xFFFF))
        out.write(f"tov={tv} toh={th} fromv={fv} fromh={fh} -> {s16(m.get(3))}\n")
    out.close()
    print(f"aim: {AIMS} cases")


if __name__ == "__main__":
    main()
