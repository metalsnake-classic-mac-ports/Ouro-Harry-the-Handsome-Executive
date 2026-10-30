"""Record turrets (1900-1999), reversors (1850-1899) and boulders
(6000-6099) at their places in Episode 1 (and nearby).

turret.txt: SetupTurretSprite with Harry somewhere around, then frames of
HandleTurretSprite (Harry walking, steps), each followed at random by
HitTurretSprite with a player shot or an explosion.
reversor.txt: SetupReversorSprite, then frames of HandleReversorSprite and
CheckTileCollisions (the reversor alone), each followed at random by
HitReversorSprite with a partner: barrel, plant, door, shot, robot.
boulder.txt: SetupBoulderSprite, then frames of HandleBoulderSprite and
CheckTileCollisions, each followed at random by HitBoulderSprite with a
robot.

SATNewSprite is stubbed (a zeroed record, pos and fix as
SetupEnemyShotSprite leaves them), GenerateSprite and EasyExplosion
logged.
Each line: "<name=value ...> -> <dump> [|| <partner>] | <outputs>"
Usage: python3 -I emu_turret.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from sprites import Faces, tvectors, dump, face_region  # noqa: E402
from emu_mask import masks  # noqa: E402

SAT, LEVEL_H, WALLS, OBSTACLE, PHYSICS = 0x10065B70, 0x10065B64, 0x100658D4, 0x100658D0, 0x100659D8
PLAYER_H, PLAYER_V, SCORE = 0x100659B4, 0x100659B0, 0x10065AF8
ROBOT_MASK = 0x10066180
FLAGS = (0x10066184, 0x10066188)
SOUNDS = {0x10065BFC: 600, 0x10065BF8: 601, 0x10065BF4: 602, 0x10065BE4: 605, 0x10065BD8: 801, 0x10065C20: 402}
RUNS = 6000


def s16(x):
    x &= 0xFFFF
    return x - 0x10000 if x & 0x8000 else x


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m, work)
    m.stub_routine(0x1005F340, lambda mm: mm.alloc(0x40))
    for f in ("InitTurretSprite", "InitReversorSprite", "InitBoulderSprite", "InitRobotSprite", "InitTileHotRects",
              "InitBarrelSprite", "InitPlantSprite", "InitDoorSprite"):
        m.call(f)
    tv = tvectors(f"{work}/ppc")
    by_name = {v: k for k, v in tv.items()}
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
    robot_mask = face_region(work, 4251)              # cicn 598 with the faces' mirrored bits
    ms = masks(work)
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
                _, kind, hh, vv = made[e]
                out.append("shot:%d:%d:%d:%d:%d:%d:%d:%d" % (
                    kind, hh, vv, s16(m.r16(e + 0x4E)), s16(m.r16(e + 0x50)), s16(m.r16(e + 0x66)),
                    struct.unpack(">i", m.read(e + 0x5E, 4))[0], struct.unpack(">i", m.read(e + 0x62, 4))[0]))
            else:
                out.append(e)
        return ",".join(out) or "-"

    places = {}
    for lvl in range(1, 16):
        data = open(f"{work}/res/Episode_1/Hlvl_{lvl}.bin", "rb").read()
        for i in range(256):
            e = data[0x10000 + i * 16:0x10000 + i * 16 + 16]
            kind = struct.unpack(">h", e[2:4])[0]
            group = "turret" if 1900 <= kind <= 1999 else "reversor" if 1850 <= kind <= 1899 else \
                "boulder" if 6000 <= kind <= 6099 else None
            if e[0] and group:
                v, hh = struct.unpack(">hh", e[12:16])
                places.setdefault(group, []).append((lvl, i, kind, v - 6, hh - 6))
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

    fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())
    sp, other = m.alloc(0xA2), m.alloc(0xA2)
    rnd = random.Random("turret")
    for group in ("turret", "reversor", "boulder"):
        out = open(f"{work}/diff/{group}.txt", "w")
        out.write(f"# {group}: name=value inputs -> dump | entry score events\n")
        for _ in range(RUNS):
            lvl, index, kind, v, hh = rnd.choice(places[group])
            data, base = use_level(lvl)
            mark = m.heap
            regions()
            m.write(base + 0x10000, data[0x10000:0x11000])
            st = {"level": lvl, "index": index, "kind": kind,
                  "v": v + rnd.choice([0, 0, rnd.randint(-60, 60)]), "h": hh + rnd.choice([0, 0, rnd.randint(-60, 60)]),
                  "pv": v + rnd.randint(-250, 250), "ph": hh + rnd.randint(-250, 250),
                  "frames": rnd.randint(1, 150), "seed": rnd.randint(1, 0x7FFFFFFE)}
            if group == "turret":
                st["kind"] = rnd.choice([kind, kind, rnd.choice([1901, 1902, 1903, 1904, 1912, 1914, 1921, 1923])])
                st["hp"] = rnd.choice([-1, -1, 1, 5])
                st["info3"] = rnd.choice([0, 0, 702])
                m.w16(base + 0x10000 + index * 16 + 10, st["info3"])
            elif group == "boulder":
                st["kind"] = rnd.choice([kind, kind, rnd.choice([6001, 6002, 6003, 6004, 6011, 6012, 6013, 6014])])
                st["info0"], st["info1"], st["info2"] = rnd.choice([0, 0, 5]), rnd.choice([0, 0, 3, 8]), rnd.choice([0, 0, -1])
                if st["info2"] == -1:
                    st["info2"] = (v if st["kind"] in (6001, 6002, 6011, 6012) else hh) + rnd.randint(-30, 30)
                for k in range(3):
                    m.w16(base + 0x10000 + index * 16 + 4 + 2 * k, st[f"info{k}"])
            else:
                st["kind"] = rnd.choice([kind, kind, rnd.choice([1851, 1852, 1861, 1862])])
                st["air"] = rnd.choice([0, 0, 0, 30])
            steps = []
            for _f in range(st["frames"]):
                hit = "-"
                if rnd.random() < 0.1:
                    hit = rnd.choice({"turret": ["shot", "fx", "fx0"], "boulder": ["robot", "plant"],
                                      "reversor": ["barrel", "plant", "door", "shot", "eshot", "robot"]}[group])
                steps.append(f"{rnd.randint(-5, 5)}:{rnd.randint(-5, 5)}:{hit}")
            st["steps"] = ",".join(steps)
            m.w16(m.glob(PLAYER_H), st["ph"])
            m.w16(m.glob(PLAYER_V), st["pv"])
            m.w32(m.glob(SCORE), 0)
            for f in FLAGS:
                m.write(m.glob(f), b"\0")
            m.write(sp, bytes(0xA2))
            m.w16(sp, st["kind"])
            m.w16(sp + 2, st["v"])
            m.w16(sp + 4, st["h"])
            m.rand_seed = st["seed"]
            m.call({"turret": "SetupTurretSprite", "reversor": "SetupReversorSprite", "boulder": "SetupBoulderSprite"}[group], sp)
            m.w16(sp + 0x68, index)
            if group == "turret" and st["hp"] != -1:
                m.w16(sp + 0x72, st["hp"])
            if group == "reversor":
                m.w16(sp + 0x8E, st["air"])
            evs = []
            for step in steps:
                dh, dv, hit = step.split(":")
                m.w16(m.glob(PLAYER_H), m.r16(m.glob(PLAYER_H)) + int(dh))
                m.w16(m.glob(PLAYER_V), m.r16(m.glob(PLAYER_V)) + int(dv))
                ev.clear()
                news.clear()
                task = {"turret": "HandleTurretSprite", "reversor": "HandleReversorSprite", "boulder": "HandleBoulderSprite"}[group]
                m.call(task, sp, regs={30: sp})
                if group != "turret":
                    m.w32(sat + 0x78, sp)
                    m.w32(sp + 0x2E, 0)
                    m.call("CheckTileCollisions")
                if hit != "-":
                    m.write(other, bytes(0xA2))
                    ov, oh = s16(m.r16(sp + 0x5A)) + 4, s16(m.r16(sp + 0x5C)) + 4
                    m.w16(other + 2, ov)
                    m.w16(other + 4, oh)
                    if hit in ("barrel", "plant", "door"):
                        m.w16(other, {"barrel": 550, "plant": 601, "door": 901}[hit])
                        m.call({"barrel": "SetupBarrelSprite", "plant": "SetupPlantSprite", "door": "SetupDoorSprite"}[hit], other)
                    else:
                        t = {"shot": "HandlePlayerShotSprite", "eshot": "HandleEnemyShotSprite", "fx": "HandleEffectSprite",
                             "fx0": "HandleEffectSprite", "robot": "HandleRobotSprite"}[hit]
                        m.w32(other + 0x1A, by_name[t])
                        if hit == "fx":
                            m.write(other + 6, struct.pack(">hhhh", 4, 4, 60, 60))
                        if hit in ("shot", "eshot"):
                            m.w16(other + 0x50, 700)
                            m.w16(other + 0x4E, -300)
                            m.w32(other + 0x5E, oh << 8)
                            m.w32(other + 0x62, ov << 8)
                        if hit == "robot":
                            m.w16(other + 0x72, 3)
                    m.w16(other + 0x5A, ov)
                    m.w16(other + 0x5C, oh)
                    m.call({"turret": "HitTurretSprite", "reversor": "HitReversorSprite", "boulder": "HitBoulderSprite"}[group], sp, other)
                    ev.append("o:%d:%d:%d:%d:%d:%d" % (m.read(other + 0x92, 1)[0], s16(m.r16(other + 0x72)), s16(m.r16(other + 0x66)),
                                                        s16(m.r16(other + 0x50)), s16(m.r16(other + 0x4E)),
                                                        struct.unpack(">i", m.read(other + 0x5E, 4))[0]))
                evs.append(frame_events())
            e = base + 0x10000 + index * 16
            o = (f"entry={m.read(e, 1)[0]}:{s16(m.r16(e + 2))}:{s16(m.r16(e + 4))}:{s16(m.r16(e + 6))}:{s16(m.r16(e + 8))} "
                 f"score={m.r32(m.glob(SCORE))} events={';'.join(evs)}")
            out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} | {o}\n")
            m.heap = mark
        out.close()
        print(f"{group}: {RUNS} cases")


if __name__ == "__main__":
    main()
