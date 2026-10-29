"""Record scientists (1100-1299) and blobs (1700-1799) at their places in
Episode 1 (and nearby).

scientist.txt: SetupScientistSprite, then frames of HandleScientistSprite
with Harry (gPlayerH/V) and the view moving, and HitScientistSprite with
a player shot at random frames. SATNewSprite is stubbed (a zeroed record,
pos and fix as SetupEnemyShotSprite leaves them).
blob.txt: SetupBlobSprite, then frames of HandleBlobSprite and
CheckTileCollisions (the blob alone), each followed at random by
HitBlobSprite with a partner: an explosion, a plant, a door, a cooler,
Harry kicking.

Each line: "<name=value ...> -> <dump> | <outputs>"
Usage: python3 -I emu_blob.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from sprites import Faces, tvectors, dump  # noqa: E402
from emu_mask import masks  # noqa: E402
from cicn import decode as cicn_decode  # noqa: E402

SAT, LEVEL_H, WALLS, OBSTACLE, PHYSICS, CONTACT = 0x10065B70, 0x10065B64, 0x100658D4, 0x100658D0, 0x100659D8, 0x10065E1C
PLAYER_H, PLAYER_V, VIEW, SCORE, FAVOR = 0x100659B4, 0x100659B0, 0x100659BC, 0x10065AF8, 0x1006856E
BLOB_MASK = 0x10065F10
FLAGS = (0x10065EFC, 0x10065F00)
SOUNDS = {0x10065BFC: 600, 0x10065BE8: 604, 0x10065BEC: 603, 0x10065C18: 300, 0x10065BB0: 4700}
RUNS = 8000


def s16(x):
    x &= 0xFFFF
    return x - 0x10000 if x & 0x8000 else x


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m, work)
    m.stub_routine(0x1005F340, lambda mm: mm.alloc(0x40))
    for f in ("InitScientistSprite", "InitBlobSprite", "InitTileHotRects", "InitPlayerKickRects", "InitPlantSprite",
              "InitDoorSprite", "InitCoolerSprite"):
        m.call(f)
    tv = tvectors(f"{work}/ppc")
    by_name = {v: k for k, v in tv.items()}
    ev, news = [], []
    m.stub_routine("STPlayRegSound", lambda mm: ev.append("p:%d" % mm.get(3)) or 0)
    m.stub_routine("STPlay3DSound", lambda mm: ev.append("3d:%d" % mm.get(3)) or 0)
    m.stub_routine("GenerateSprite", lambda mm: ev.append("gen:%d:%d:%d" % (s16(mm.get(5)), s16(mm.get(3)), s16(mm.get(4)))) or 0)
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
    blob_face = m.r32(m.glob(BLOB_MASK))
    w, h, mk, _, _ = cicn_decode(open(f"{work}/res/Harry_Graphics/cicn_598.bin", "rb").read())
    blob_mask = {(x, y) for y in range(h) for x in range(w) if mk[y][x]}
    ms = masks(work)
    physics, contact, view = m.glob(PHYSICS), m.glob(CONTACT), m.glob(VIEW)
    for i, v in enumerate((50, 70, 245, 252, 150)):
        m.w16(physics + 2 * i, v)

    def regions():
        m.regions.clear()
        for k in range(32):
            m.w32(walls + 0x1A + k * 0x2E, m.make_region(ms[f"wall{k}"]))
        m.w32(obstacle + 0x1A, m.make_region(ms["obstacle"]))
        m.w32(blob_face + 0x14, m.make_region(blob_mask))

    def frame_events():
        out = []
        made = {n[0]: n for n in news}
        for e in ev:
            if isinstance(e, int):
                _, kind, hh, vv = made[e]
                out.append("shot:%d:%d:%d:%d:%d:%d:%d:%d:t%d" % (
                    kind, hh, vv, s16(m.r16(e + 0x4E)), s16(m.r16(e + 0x50)), s16(m.r16(e + 0x66)),
                    struct.unpack(">i", m.read(e + 0x5E, 4))[0], struct.unpack(">i", m.read(e + 0x62, 4))[0],
                    s16(m.r16(e + 0x74))))
            else:
                out.append(e)
        return ",".join(out) or "-"

    places = {}
    for lvl in range(1, 16):
        data = open(f"{work}/res/Episode_1/Hlvl_{lvl}.bin", "rb").read()
        for i in range(256):
            e = data[0x10000 + i * 16:0x10000 + i * 16 + 16]
            kind = struct.unpack(">h", e[2:4])[0]
            group = "scientist" if 1100 <= kind <= 1299 else "blob" if 1700 <= kind <= 1799 else None
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
    rnd = random.Random("blob")
    for group in ("scientist", "blob"):
        out = open(f"{work}/diff/{group}.txt", "w")
        out.write(f"# {group}: name=value inputs -> dump | entry score favor contact events\n")
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
            st["vv"], st["vh"] = st["pv"] - 200 + rnd.randint(-150, 150), st["ph"] - 200 + rnd.randint(-150, 150)
            if group == "scientist":
                st["kind"] = rnd.choice([kind, kind, rnd.choice([1111, 1121, 1131, 1141, 1211, 1221, 1231, 1241])])
                st["mode"] = rnd.choice([-1, -1, -1, 9, 21, 45])
            else:
                st["kind"] = rnd.choice([kind, kind, rnd.choice([1701, 1711, 1721])])
                st["hp"] = rnd.choice([-1, -1, -1, 0, 1])
                st["air"] = rnd.choice([0, 0, 0, 30])
                st["info3"] = rnd.choice([0, 0, 702])
                m.w16(base + 0x10000 + index * 16 + 10, st["info3"])
            steps = []
            for _f in range(st["frames"]):
                hit = "-"
                if rnd.random() < 0.08:
                    hit = rnd.choice({"scientist": ["shot", "shotf"],
                                      "blob": ["fx", "fx0", "plant", "door", "cooler", "kick", "kick"]}[group])
                steps.append(f"{rnd.randint(-5, 5)}:{rnd.randint(-5, 5)}:{hit}")
            st["steps"] = ",".join(steps)
            m.w16(m.glob(PLAYER_H), st["ph"])
            m.w16(m.glob(PLAYER_V), st["pv"])
            m.w16(view, st["vv"])
            m.w16(view + 2, st["vh"])
            m.w32(m.glob(SCORE), 0)
            m.w16(FAVOR, 100)
            m.w16(contact, 0)
            for f in FLAGS:
                m.write(m.glob(f), b"\0")
            m.write(sp, bytes(0xA2))
            m.w16(sp, st["kind"])
            m.w16(sp + 2, st["v"])
            m.w16(sp + 4, st["h"])
            m.rand_seed = st["seed"]
            m.call("SetupScientistSprite" if group == "scientist" else "SetupBlobSprite", sp)
            m.w16(sp + 0x68, index)
            if group == "scientist" and st["mode"] != -1:
                m.w32(sp + 0x6E, st["mode"])
            if group == "blob":
                if st["hp"] != -1:
                    m.w16(sp + 0x72, st["hp"])
                m.w16(sp + 0x8E, st["air"])
            evs = []
            for step in steps:
                dh, dv, hit = step.split(":")
                for a, d in ((m.glob(PLAYER_H), int(dh)), (m.glob(PLAYER_V), int(dv)), (view, int(dv)), (view + 2, int(dh))):
                    m.w16(a, m.r16(a) + d)
                ev.clear()
                news.clear()
                if group == "scientist":
                    m.call("HandleScientistSprite", sp)
                else:
                    m.call("HandleBlobSprite", sp, regs={30: sp})
                    m.w32(sat + 0x78, sp)
                    m.w32(sp + 0x2E, 0)
                    m.call("CheckTileCollisions")
                if hit != "-":
                    m.write(other, bytes(0xA2))
                    ov, oh = s16(m.r16(sp + 0x5A)) + 4, s16(m.r16(sp + 0x5C)) + 4
                    if hit == "kick":
                        ov, oh = ov - 30, oh - 30
                    m.w16(other + 2, ov)
                    m.w16(other + 4, oh)
                    if hit in ("plant", "door", "cooler"):
                        m.w16(other, {"plant": 601, "door": 902, "cooler": 500}[hit])
                        m.call({"plant": "SetupPlantSprite", "door": "SetupDoorSprite", "cooler": "SetupCoolerSprite"}[hit], other)
                    else:
                        t = {"shot": "HandlePlayerShotSprite", "shotf": "HandlePlayerShotSprite", "fx": "HandleEffectSprite",
                             "fx0": "HandleEffectSprite", "kick": "HandlePlayerSprite"}[hit]
                        m.w32(other + 0x1A, by_name[t])
                        if hit == "fx":
                            m.write(other + 6, struct.pack(">hhhh", 4, 4, 60, 60))
                        if hit == "shotf":
                            m.write(other + 0x92, b"\1")
                        if hit == "kick":
                            m.w16(other + 0x66, int(dh) % 36)
                    m.w16(other + 0x5A, ov)
                    m.w16(other + 0x5C, oh)
                    m.call("HitScientistSprite" if group == "scientist" else "HitBlobSprite", sp, other)
                    ev.append("o:%d" % m.read(other + 0x92, 1)[0])
                evs.append(frame_events())
            e = base + 0x10000 + index * 16
            o = (f"entry={m.read(e, 1)[0]} score={m.r32(m.glob(SCORE))} favor={s16(m.r16(FAVOR))} "
                 f"contact={s16(m.r16(contact))} events={';'.join(evs)}")
            out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} | {o}\n")
            m.heap = mark
        out.close()
        print(f"{group}: {RUNS} cases")


if __name__ == "__main__":
    main()
