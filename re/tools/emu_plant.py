"""Record the plant (601) moving: SetupPlantSprite, then frames of
HandlePlantSprite (animation, HandleGlide, HandleHalfPipe, JumpScale,
the position) each followed by CheckTileCollisions with the plant as the
only sprite (HitPlantTileSprite: WallBounce / BGTileBounce with the
plant's mask, cicn 599) - on the real levels, weighted to obstacles, wall
edges and the special floors.

Each line: "<name=value ...> -> <plant dump> | flag=<PTR_DAT_10066118> sounds=<n>"
Usage: python3 -I emu_plant.py [re/work]
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

SAT, WALLS, OBSTACLE, PLANT_MASK, PHYSICS, FLAG = 0x10065B70, 0x100658D4, 0x100658D0, 0x10065ED0, 0x100659D8, 0x10066118
CASES_PER_LEVEL = 800


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m)
    m.stub_routine(0x1005F340, lambda mm: mm.alloc(0x40))      # the mask loader; filled below
    m.call("InitPlantSprite")
    m.call("InitTileHotRects")
    tv = tvectors(f"{work}/ppc")
    ms = masks(work)
    w, h, mk, _, _ = cicn_decode(open(f"{work}/res/Harry_Graphics/cicn_599.bin", "rb").read())
    plant_mask = {(x, y) for y in range(h) for x in range(w) if mk[y][x]}
    sounds = []
    for f in ("STPlayRegSound", "STPlay3DSound", "STPlay3DSoundPitched"):
        m.stub_routine(f, 0, sounds)
    for f in ("GammaFadeOut", "GammaFadeIn", 0x1002549C, "AddFGRedraw"):
        m.stub_routine(f, 0)
    m.stub_routine(0x10056DE8, 0)
    sat = m.glob(SAT)
    walls = m.alloc(64 * 0x2E)
    m.w32(m.glob(WALLS), walls)
    obstacle = m.alloc(0x40)
    m.w32(m.glob(OBSTACLE), obstacle)
    plant_face = m.r32(m.glob(PLANT_MASK))
    physics, flag = m.glob(PHYSICS), m.glob(FLAG)
    for i, v in enumerate((50, 70, 245, 252, 150)):
        m.w16(physics + 2 * i, v)
    sp = m.alloc(0xA2)
    rnd = random.Random("plant")
    out = open(f"{work}/diff/plant.txt", "w")
    out.write("# plant: name=value inputs -> dump | flag sounds\n")
    n = 0
    for lvl in range(1, 16):
        data = open(f"{work}/res/Episode_1/Hlvl_{lvl}.bin", "rb").read()
        m.load_level(data)
        mark = m.heap
        special = cells(data, set(range(1, 16)))
        grid = lambda x, y: struct.unpack(">H", data[0x8000 + (x * 128 + y) * 2:0x8000 + (x * 128 + y) * 2 + 2])[0]
        edges = [(x, y) for x in range(1, 127) for y in range(1, 127) if grid(x, y) == 0xFFFF and any(
            grid(a, b) != 0xFFFF for a, b in ((x, y - 1), (x + 1, y), (x - 1, y), (x, y + 1)))]
        for _ in range(CASES_PER_LEVEL):
            m.heap = mark
            m.regions.clear()
            for k in range(32):
                m.w32(walls + 0x1A + k * 0x2E, m.make_region(ms[f"wall{k}"]))
            m.w32(obstacle + 0x1A, m.make_region(ms["obstacle"]))
            m.w32(plant_face + 0x1A, m.make_region(plant_mask))
            pool = rnd.choice([special, edges, edges]) or edges
            x, y = rnd.choice(pool) if pool else (rnd.randint(2, 125), rnd.randint(2, 125))
            st = {"level": lvl, "h": x * 32 - 18 + rnd.randint(-20, 20), "v": y * 32 - 18 + rnd.randint(-20, 20),
                  "speedh": rnd.choice([0, rnd.randint(-30, 30), rnd.randint(-2500, 2500)]),
                  "speedv": rnd.choice([0, rnd.randint(-30, 30), rnd.randint(-2500, 2500)]),
                  "dir": rnd.choice([0, 0, rnd.randint(-1, 8)]),
                  "air": rnd.choice([0, 0, 0, 0, 1, 20, 30, 216, 999, 1000, 1029, 1030, 1044]),
                  "slide": rnd.choice([0, 0, rnd.randint(0, 15), 87, 88, 100]), "frames": rnd.randint(1, 6),
                  "seed": rnd.randint(1, 0x7FFFFFFE)}
            m.write(sp, bytes(0xA2))
            m.w16(sp, 601)
            m.w16(sp + 2, st["v"])
            m.w16(sp + 4, st["h"])
            m.call("SetupPlantSprite", sp)
            for off, k in ((0x50, "speedh"), (0x4E, "speedv"), (0x66, "dir"), (0x8E, "air"), (0x9C, "slide")):
                m.w16(sp + off, st[k])
            m.w32(sp + 0x7E, 0)
            pface = m.alloc(0x1C)
            m.write(pface, struct.pack(">hhhh", 0, 0, 36, 36))
            m.w16(flag, 0x5555)
            m.rand_seed = st["seed"]
            sounds.clear()
            for _f in range(st["frames"]):
                m.call("HandlePlantSprite", sp, regs={30: 0x1007DFAC})
                # CheckTileCollisions needs a face for the centre: keep the
                # real face, with known bounds
                real = m.r32(sp + 0x7E)
                m.w32(sp + 0x7E, pface)
                m.w32(sat + 0x78, sp)
                m.w32(sp + 0x2E, 0)
                m.call("CheckTileCollisions")
                m.w32(sp + 0x7E, real)
            fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())
            out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} | flag={m.r16(flag)} sounds={len(sounds)}\n")
            n += 1
    out.close()
    print(f"plant: {n} cases")


if __name__ == "__main__":
    main()
