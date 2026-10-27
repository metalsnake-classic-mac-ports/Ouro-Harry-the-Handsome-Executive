"""Record CheckTileCollisions with the player in SAT's sprite list: the
original's HitPlayerTileSprite reacts to the tiles (WallBounce,
RectBounceFake, BGTileBounce, the region mask tests, the kick test with the
kick-pose masks, damage / healing / water). Real levels, real masks, the
game's own tables (InitTileHotRects, InitPlayerKickRects,
LoadTileDefinitions). Stubbed: sounds, Ouch (counted), AddFGRedraw.

One line per case:
  level atV atH speedH speedV dir pose hurt wet wallFlag bgFlag cooldown
  -> atV atH posV posH fixH fixV speedH speedV wallFlag bgFlag cooldown
     comfortDelta wet floorKind kickContact ouches sounds

Usage: python3 -I emu_tiles.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from emu_mask import masks  # noqa: E402
from emu_bounce import bg_cases  # noqa: E402

CASES = 15000
TV_HIT_PLAYER_TILE = 0x10066FBC          # TVector of HitPlayerTileSprite
SLOT = {"sat": 0x10065B70, "body": 0x10065EA0, "walls": 0x100658D4, "obstacle": 0x100658D0,
        "physics": 0x100659D8, "debug": 0x10065B74, "wall_flag": 0x10065E58, "bg_flag": 0x10065E54,
        "cooldown": 0x10065E44, "comfort": 0x10065E6C, "wet": 0x10065E8C, "contact": 0x10065E1C,
        "caffeine": 0x10065AD4}
FLOOR = 0x10068578


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    ms = masks(work)
    # the kick-pose face masks as the original builds them (emu_mask.py faces)
    faces = {}
    lines = open(f"{work}/diff/facemasks.txt").read().splitlines()
    i = 1
    while i < len(lines):
        _, fid, w, h = lines[i].split()
        fid, w, h = int(fid), int(w), int(h)
        faces[fid] = {(x, y) for y in range(h) for x in range(w) if lines[i + 1 + y][x] == "#"}
        i += 1 + h
    m = Machine(f"{work}/ppc")
    m.call("InitTileHotRects")
    m.call("InitPlayerKickRects")
    g = {k: m.glob(a) for k, a in SLOT.items()}
    ouches, sounds = [], []
    m.stub_routine("Ouch", 0, ouches)
    m.stub_routine("STPlay3DSoundPitched", 0, sounds)
    m.stub_routine("STPlayRegSound", 0, sounds)
    m.stub_routine(0x10056DE8, 0)                   # "is this sound playing": no
    m.stub_routine("AddFGRedraw", 0)
    sat = g["sat"]                                  # SAT's globals; first sprite at +0x78
    walls = m.alloc(64 * 0x2E)
    m.w32(g["walls"], walls)
    obstacle_face = m.alloc(0x40)
    m.w32(g["obstacle"], obstacle_face)
    body_face = m.alloc(0x40)
    m.w32(g["body"], body_face)
    m.write(g["debug"], b"\0")                      # debug keys off
    for i, v in enumerate((50, 70, 245, 252, 150)):
        m.w16(g["physics"] + 2 * i, v)
    sp = m.alloc(0xA2)
    face = m.alloc(0x1C)
    rnd = random.Random("tiles")
    out = open(f"{work}/diff/tiles.txt", "w")
    out.write("# " + __doc__.split("One line per case:")[1].split("Usage")[0].strip().replace("\n", " ") + "\n")
    n = 0
    last_lvl, mark = None, 0
    def special_cases(per=300):
        """Extra cases on the special floors (background kinds 2-6) with
        wetness around the water sound's threshold (171)."""
        for lvl in range(1, 16):
            data = open(f"{work}/res/Episode_1/Hlvl_{lvl}.bin", "rb").read()
            m.load_level(data)
            bgk = [(data[0x12094 + i // 2] >> (4 if i % 2 == 0 else 0)) & 15 for i in range(96)]
            cells = [(x, y) for x in range(1, 127) for y in range(1, 127)
                     if (lambda b: b < 96 and 2 <= bgk[b] <= 6)(struct.unpack(">H", data[(x * 128 + y) * 2:(x * 128 + y) * 2 + 2])[0])]
            for _ in range(per if cells else 0):
                x, y = rnd.choice(cells)
                yield lvl, None, (y * 32, x * 32), m

    def all_cases():
        for i, c in enumerate(bg_cases(m, rnd, work, per=CASES // 15)):
            yield 1 + i // (CASES // 15), c[0], c[1], c[2]
        yield from special_cases()

    for lvl, kinds, (tv, th), _ in all_cases():
        n += 1
        # regions are made anew for each case: reuse the heap above the level
        if lvl != last_lvl:
            last_lvl, mark = lvl, m.heap
        m.heap = mark
        m.regions.clear()
        m.w32(body_face + 0x1A, m.make_region(ms["body"]))
        m.w32(obstacle_face + 0x1A, m.make_region(ms["obstacle"]))
        for k in range(32):
            m.w32(walls + 0x1A + k * 0x2E, m.make_region(ms[f"wall{k}"]))
        av, ah = tv - 32 + rnd.randint(-40, 40), th - 32 + rnd.randint(-40, 40)
        sh, sv = rnd.randint(-2500, 2500), rnd.randint(-2500, 2500)
        d = rnd.randint(0, 35)
        kicking = rnd.random() < 0.3
        pose = 108 if kicking else rnd.choice([0, 36, 72])
        hurt = rnd.choice([0, 0, 0, 0, rnd.randint(1, 30)])
        wet = rnd.choice([0, 0, rnd.randint(0, 200), rnd.randint(165, 176)])
        wall_flag, bg_flag = rnd.choice([0, 0, 1]), rnd.choice([0, 0, 1])
        cooldown = rnd.choice([0, 0, rnd.randint(1, 15)])
        m.write(sp, bytes(0xA2))
        for off, v in ((0x02, av), (0x04, ah), (0x5A, av), (0x5C, ah), (0x50, sh), (0x4E, sv),
                       (0x66, d), (0x8E, hurt)):
            m.w16(sp + off, v)
        m.write(sp + 0x06, struct.pack(">hhhh", *((0, 0, 64, 64) if kicking else (22, 22, 42, 42))))
        m.w32(sp + 0x5E, 0x7777)
        m.w32(sp + 0x62, 0x7777)
        m.w32(sp + 0x9E, TV_HIT_PLAYER_TILE)
        m.write(face, struct.pack(">hhhh", 0, 0, 64, 64) + bytes(0x14))
        frame = d + pose
        if frame >= 107:
            m.w32(face + 0x14, m.make_region(faces[1101 + frame]))
        m.w32(sp + 0x7E, face)
        m.w32(sat + 0x78, sp)
        m.w32(sp + 0x2E, 0)
        m.write(g["wall_flag"], bytes([wall_flag]))
        m.write(g["bg_flag"], bytes([bg_flag]))
        for k, v in (("cooldown", cooldown), ("comfort", 0), ("wet", wet), ("contact", 0), ("caffeine", 0)):
            m.w16(g[k], v)
        m.w16(FLOOR, 1)
        ouches.clear()
        sounds.clear()
        m.call("CheckTileCollisions")
        outs = [m.r16(sp + 0x5A), m.r16(sp + 0x5C), m.r16(sp + 0x02), m.r16(sp + 0x04),
                struct.unpack(">i", m.read(sp + 0x5E, 4))[0], struct.unpack(">i", m.read(sp + 0x62, 4))[0],
                m.r16(sp + 0x50), m.r16(sp + 0x4E), m.read(g["wall_flag"], 1)[0], m.read(g["bg_flag"], 1)[0],
                m.r16(g["cooldown"]), m.r16(g["comfort"]), m.r16(g["wet"]), m.r16(FLOOR), m.r16(g["contact"]),
                len(ouches), len(sounds)]
        ins = [lvl, av, ah, sh, sv, d, pose, hurt, wet, wall_flag, bg_flag, cooldown]
        out.write(f"{' '.join(map(str, ins))} -> {' '.join(map(str, outs))}\n")
    out.close()
    print(f"tiles: {n} cases")


if __name__ == "__main__":
    main()
