"""Record HitPlantSprite - the plant's hit task - with Harry (kicking or
walking into it), another plant, a water cooler, a door, and player /
enemy shots. Every sprite is made by its real setup.

Each line: "<name=value ...> -> <plant dump> || <other dump> | contact=<kick contact> events=<...>"
  other   1 Harry, 2 plant, 3 cooler, 4 door, 5 player shot, 6 enemy shot,
          7 explosion (kind okind, frame odir; blast cooldown "blast"),
          8 barrel, 9 chair, 10 ramp (kind okind), 11 collider (kind okind),
          12 stairs (kind okind), 13 sliding door, 14 co-worker (kind okind)
  events  p:<snd> (STPlayRegSound), 3d:<snd> (STPlay3DSound)
Usage: python3 -I emu_planthit.py [re/work]
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

BODY_FACE, PLANT_MASK, CHAIR_MASK, BARREL_MASK, PHYSICS, CONTACT = 0x10065EA0, 0x10065ED0, 0x10065ECC, 0x10065EF0, 0x100659D8, 0x10065E1C
TV_PLAYER, TV_PLAYER_SHOT, TV_ENEMY_SHOT, TV_COLLIDER = 0x10066FC4, 0x100672FC, 0x100671BC, 0x100670E4
SOUNDS = {0x10065BB8: 1101, 0x10065BBC: 1100, 0x10065BD8: 801, 0x10065BDC: 800, 0x10065C2C: 258, 0x10065C30: 257}
DOORS = [901, 902, 903, 911, 912, 913, 921, 922, 923, 931, 932, 933]
CASES = 30000


def cicn_mask(work, n):
    w, h, mk, _, _ = cicn_decode(open(f"{work}/res/Harry_Graphics/cicn_{n}.bin", "rb").read())
    return {(x, y) for y in range(h) for x in range(w) if mk[y][x]}


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m)
    m.stub_routine(0x1005F340, lambda mm: mm.alloc(0x40))
    for f in ("InitPlantSprite", "InitCoolerSprite", "InitDoorSprite", "InitPlayerKickRects", "InitEffectSprite",
              "InitBarrelSprite", "InitChairSprite", "InitRampSprite", "InitStairsSprite", "InitSlidingDoorSprite", "InitCoWorkerSprite"):
        m.call(f)
    tv = tvectors(f"{work}/ppc")
    body = masks(work)["body"]
    plant_mask, barrel_mask = cicn_mask(work, 599), cicn_mask(work, 598)
    ev = []
    m.stub_routine("STPlayRegSound", lambda mm: ev.append("p:%d" % mm.get(3)) or 0)
    m.stub_routine("STPlay3DSound", lambda mm: ev.append("3d:%d" % mm.get(3)) or 0)
    busy = [0]
    m.stub_routine(0x10056DE8, lambda mm: busy[0])
    for slot, snd in SOUNDS.items():
        m.w32(m.glob(slot), snd)
    body_face = m.alloc(0x40)
    m.w32(m.glob(BODY_FACE), body_face)
    plant_face = m.r32(m.glob(PLANT_MASK))
    barrel_face = m.r32(m.glob(BARREL_MASK))
    chair_face = m.r32(m.glob(CHAIR_MASK))
    physics, contact = m.glob(PHYSICS), m.glob(CONTACT)
    for i, v in enumerate((50, 70, 245, 252, 150)):
        m.w16(physics + 2 * i, v)
    sp, other = m.alloc(0xA2), m.alloc(0xA2)
    mark = m.heap
    rnd = random.Random("planthit")
    out = open(f"{work}/diff/planthit.txt", "w")
    out.write("# HitPlantSprite: name=value inputs -> plant dump || other dump | contact events\n")
    for case in range(CASES):
        m.heap = mark
        m.regions.clear()
        m.w32(body_face + 0x1A, m.make_region(body))
        m.w32(plant_face + 0x1A, m.make_region(plant_mask))
        m.w32(barrel_face + 0x1A, m.make_region(barrel_mask))
        m.w32(chair_face + 0x1A, m.make_region(plant_mask))
        v, h = rnd.randint(200, 3800), rnd.randint(200, 3800)
        kind = rnd.choice([1, 1, 1, 2, 2, 3, 4, 4, 5, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 12, 12, 13, 14])
        st = {"v": v, "h": h, "other": kind, "okind": 0,
              "ov": v + rnd.randint(-40, 40), "oh": h + rnd.randint(-40, 40), "samefix": int(rnd.random() < 0.1),
              "speedh": rnd.randint(-2500, 2500), "speedv": rnd.randint(-2500, 2500),
              "ospeedh": rnd.randint(-2500, 2500), "ospeedv": rnd.randint(-2500, 2500),
              "air": rnd.choice([0, 0, 0, 0, 5]), "oair": rnd.choice([0, 0, 0, 0, 5]),
              "frozen": int(rnd.random() < 0.03), "ofrozen": int(rnd.random() < 0.03),
              "kicking": int(rnd.random() < 0.4), "odir": rnd.randint(0, 35), "busy": rnd.choice([0, 0, 1]),
              "contact": rnd.choice([0, 64]), "seed": rnd.randint(1, 0x7FFFFFFE)}
        if kind == 4:
            st["okind"] = rnd.choice(DOORS)
        elif kind in (5, 6):
            st["okind"] = rnd.choice([0, 1, 9450, 9610])
        elif kind == 7:
            st["okind"] = rnd.choice([0, 14, 12, -1])
        elif kind == 10:
            st["okind"] = rnd.choice([9701, 9702, 9703, 9704])
        elif kind == 11:
            st["okind"] = rnd.choice([3100, 9200, 9600, 2301])
        elif kind == 12:
            st["okind"] = rnd.choice([9901, 9902, 9903, 9904])
        elif kind == 13:
            st["okind"] = rnd.choice([951, 961])
        elif kind == 14:
            st["okind"] = rnd.choice([1301, 1321, 1441])
        st["blast"] = rnd.choice([0, 0, 0, 1, 5, -1])
        m.write(sp, bytes(0xA2))
        m.w16(sp, 601)
        m.w16(sp + 2, v)
        m.w16(sp + 4, h)
        st["seed0"] = rnd.randint(1, 0x7FFFFFFE)
        m.rand_seed = st["seed0"]                   # the chair's setup takes a random direction
        m.call("SetupPlantSprite", sp)
        if rnd.random() < 0.5:
            m.w32(sp + 0x5E, (h << 8) + rnd.randint(0, 255))
            m.w32(sp + 0x62, (v << 8) + rnd.randint(0, 255))
        m.write(other, bytes(0xA2))
        m.w16(other, {2: 601, 3: 500, 8: 550, 9: 560}.get(kind, st["okind"]))
        m.w16(other + 2, st["ov"])
        m.w16(other + 4, st["oh"])
        if kind == 7:
            m.call("SetupEffectSprite", other)
            m.w16(other, st["okind"])
        elif kind == 2:
            m.call("SetupPlantSprite", other)
        elif kind == 3:
            m.call("SetupCoolerSprite", other)
        elif kind == 4:
            m.call("SetupDoorSprite", other)
        elif kind in (8, 9, 10, 12, 13, 14):
            m.call({8: "SetupBarrelSprite", 9: "SetupChairSprite", 10: "SetupRampSprite", 12: "SetupStairsSprite",
                    13: "SetupSlidingDoorSprite", 14: "SetupCoWorkerSprite"}[kind], other)
            m.w16(other + 0x5A, st["ov"])
            m.w16(other + 0x5C, st["oh"])
        else:
            m.w32(other + 0x1A, {1: TV_PLAYER, 5: TV_PLAYER_SHOT, 6: TV_ENEMY_SHOT, 11: TV_COLLIDER}[kind])
            m.w16(other + 0x5A, st["ov"])
            m.w16(other + 0x5C, st["oh"])
            m.w32(other + 0x5E, (st["oh"] << 8) + rnd.randint(0, 255))
            m.w32(other + 0x62, (st["ov"] << 8) + rnd.randint(0, 255))
            if kind == 11:
                m.write(other + 6, struct.pack(">hhhh", 0, 0, rnd.randint(16, 48), rnd.randint(16, 48)))
            if kind == 1:
                m.write(other + 6, struct.pack(">hhhh", *((0, 0, 64, 64) if st["kicking"] else (22, 22, 42, 42))))
                # Harry's SAT position may lag behind his at
                m.w16(other + 2, st["ov"] + rnd.choice([0, 0, rnd.randint(-3, 3)]))
                m.w16(other + 4, st["oh"] + rnd.choice([0, 0, rnd.randint(-3, 3)]))
        if st["samefix"]:
            m.w32(other + 0x5E, struct.unpack(">i", m.read(sp + 0x5E, 4))[0] + rnd.randint(0, 255))
            m.w32(other + 0x62, struct.unpack(">i", m.read(sp + 0x62, 4))[0] + rnd.randint(0, 255))
        for s, pre in ((sp, ""), (other, "o")):
            m.w16(s + 0x50, st[pre + "speedh"])
            m.w16(s + 0x4E, st[pre + "speedv"])
            m.w16(s + 0x8E, st[pre + "air"])
            m.write(s + 0x92, bytes([st[pre + "frozen"]]))
        if kind == 7:
            st["odir"] = rnd.randint(0, 26)
        m.w16(other + 0x66, st["odir"])
        m.w16(sp + 0x96, st["blast"])
        st["f94"] = rnd.choice([0, 0, 0, 2])
        m.w16(sp + 0x94, st["f94"])
        # what the setups and the perturbations left, as inputs
        for s_, pre in ((sp, ""), (other, "o")):
            st[pre + "pv"], st[pre + "ph"] = m.r16(s_ + 2), m.r16(s_ + 4)
            st[pre + "fixh"] = struct.unpack(">i", m.read(s_ + 0x5E, 4))[0]
            st[pre + "fixv"] = struct.unpack(">i", m.read(s_ + 0x62, 4))[0]
            st[pre + "hot"] = ":".join(str(struct.unpack(">h", m.read(s_ + o, 2))[0]) for o in (6, 8, 10, 12))
            st[pre + "atv"], st[pre + "ath"] = struct.unpack(">hh", m.read(s_ + 0x5A, 4))
        m.w16(contact, st["contact"])
        busy[0] = st["busy"]
        m.rand_seed = st["seed"]
        ev.clear()
        m.call("HitPlantSprite", sp, other)
        fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())
        out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} || {dump(m, other, faces, tv)} | "
                  f"contact={m.r16(contact)} events={','.join(ev) or '-'}\n")
    out.close()
    print(f"planthit: {CASES} cases")


if __name__ == "__main__":
    main()
