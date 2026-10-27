"""Record the player's shots: the held weapon (HandleHeldWeaponSprite with
FindHeldWeaponPos, MakeNewShot, AssignShotSpeed), a new shot sprite
(SetupPlayerShotSprite) and its flight (HandlePlayerShotSprite).

Files (each line "<inputs> -> <outputs>", fields named in the first line):
  diff/weapon.txt  the held weapon; "shots" lists the new sprites as
                   kind:h:v:speedV:speedH joined by ",", "events" the
                   sounds (p:<snd>) and explosions (x:<h>:<v>:<a>:<b>)
  diff/shot.txt    SetupPlayerShotSprite, then HandlePlayerShotSprite
                   for a number of frames
  diff/shottiles.txt  CheckTileCollisions with a shot as the only sprite
                   (HitPlayerShotTileSprite, WallBounce with the shot's
                   mask, face 4021) on the real levels
Usage: python3 -I emu_shots.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402

PLAYER, PLAYER_DIR, RECOIL, HOLD, BADASS = 0x10065980, 0x10065E90, 0x10065E38, 0x10065EA4, 0x10065AE8
SHOT_KIND, CUR_SLOT, INVENTORY = 0x10068576, 0x1006857A, 0x10065AC4
WEAPON_FACES, WEAPON_KIND = 0x100660D8, 0x100660DC
SHOT_FACES0, SHOT_FACES1, SHOT_FACES4, HARRY_FACES = 0x10066144, 0x10066140, 0x10066074, 0x10065E9C
SOUNDS = {0x10065BF4: 602, 0x10065C64: 239}
ITEMS = [-1, 11299, 11499, 11599, 11699, 11799, 11899, 11999]
FACE_BASE = 0x50000000          # fake face pointers: FACE_BASE + table * 0x1000 + index
CASES = 20000


def s16(v):
    return ((v & 0xFFFF) ^ 0x8000) - 0x8000


def face_tables(m):
    """Fill the face tables with recognisable fake pointers."""
    for t, (slot, n, indirect) in enumerate([(WEAPON_FACES, 72, False), (SHOT_FACES0, 7, False),
                                             (SHOT_FACES1, 7, True), (SHOT_FACES4, 7, True)]):
        base = m.r32(slot) if indirect else m.glob(slot)
        for i in range(n):
            m.w32(base + 4 * i, FACE_BASE + t * 0x1000 + i)
    harry = m.alloc(144 * 0x1C)
    m.w32(m.glob(HARRY_FACES), harry)
    return harry


def face_name(m, p, harry):
    if p == 0:
        return "-"
    if harry <= p < harry + 144 * 0x1C:
        return "h%d" % ((p - harry) // 0x1C)
    return "t%d.%d" % ((p - FACE_BASE) >> 12, (p - FACE_BASE) & 0xFFF)


def weapon(m, work, rnd):
    ev, shots = [], []
    m.stub_routine("STPlayRegSound", lambda mm: ev.append("p:%d" % mm.get(3)) or 0)
    m.stub_routine("EasyExplosion", lambda mm: ev.append("x:%d:%d:%d:%d" % tuple(s16(mm.get(i)) for i in (3, 4, 7, 8))) or 0)

    def new_sprite(mm):
        s = mm.alloc(0xA2)
        mm.w16(s, mm.get(3))
        mm.w16(s + 2, mm.get(5))
        mm.w16(s + 4, mm.get(4))
        shots.append(s)
        return s
    m.stub_routine("SATNewSprite", new_sprite)
    for slot, snd in SOUNDS.items():
        m.w32(m.glob(slot), snd)
    harry = face_tables(m)
    player, inv = m.alloc(0xA2), m.r32(INVENTORY)
    m.w32(m.glob(PLAYER), player)
    pface = m.alloc(0x1C)
    m.w32(player + 0x7E, pface)
    g = {n: m.glob(a) for n, a in (("dir", PLAYER_DIR), ("recoil", RECOIL), ("hold", HOLD), ("badass", BADASS))}
    kind_byte = m.glob(WEAPON_KIND)
    sp = m.alloc(0xA2)
    out = open(f"{work}/diff/weapon.txt", "w")
    out.write("# HandleHeldWeaponSprite: dir recoil hold badass shotkind item atV atH scale0 scale1 "
              "faceT faceL faceB faceR frozen seed -> posV posH atV atH face kind flag8a flag8b badass shots events\n")
    for case in range(CASES):
        d = rnd.randint(0, 35)
        recoil = rnd.choice([0, 1, 2, 2, 2, 3])
        hold = rnd.choice([0, 0, 1, 30])
        badass = rnd.choice([0, 0, 1, 6, rnd.randint(0, 100)])
        kind = rnd.choice([0, 1, 2, 2, 4, 5, 6, 7, rnd.randint(-1, 10)])
        item = rnd.choice(ITEMS)
        av, ah = rnd.randint(0, 4000), rnd.randint(0, 4000)
        sc = rnd.choice([0x100, 0x100, 0x100, 0x200, rnd.randint(0x80, 0x300), -29788])
        sc1 = sc if rnd.random() < 0.8 else rnd.choice([0x100, 0x180])
        bounds = rnd.choice([(0, 0, 64, 64), (0, 0, 64, 64), (rnd.randint(-4, 8), rnd.randint(-4, 8),
                                                                 rnd.randint(40, 70), rnd.randint(40, 70))])
        frozen = 1 if rnd.random() < 0.03 else 0
        seed = rnd.randint(1, 0x7FFFFFFE)
        m.w16(g["dir"], d)
        m.w16(g["recoil"], recoil)
        m.w16(g["hold"], hold)
        m.w16(g["badass"], badass)
        m.w16(SHOT_KIND, kind)
        for i in range(12):
            m.w16(inv + 4 * i, -1)
        m.w16(inv, item)
        m.w16(CUR_SLOT, 0)
        for off, v in ((0x5A, av), (0x5C, ah), (0x52, sc), (0x54, sc1)):
            m.w16(player + off, v)
        m.write(pface, struct.pack(">hhhh", *bounds))
        m.write(sp, bytes(0xA2))
        m.write(sp + 0x92, bytes([frozen]))
        m.write(kind_byte, b"\x55")
        m.rand_seed = seed
        ev.clear()
        shots.clear()
        m.call("HandleHeldWeaponSprite", sp)
        news = ",".join("%d:%d:%d:%d:%d" % (m.r16(s), m.r16(s + 4), m.r16(s + 2), m.r16(s + 0x4E), m.r16(s + 0x50))
                        for s in shots) or "-"
        outs = [m.r16(sp + 2), m.r16(sp + 4), m.r16(sp + 0x5A), m.r16(sp + 0x5C), face_name(m, m.r32(sp + 0x7E), harry),
                m.read(kind_byte, 1)[0], m.read(sp + 0x8A, 1)[0], m.read(sp + 0x8B, 1)[0], m.r16(g["badass"]), news,
                ",".join(ev) or "-"]
        ins = [d, recoil, hold, badass, kind, item, av, ah, sc, sc1, *bounds, frozen, seed]
        out.write(f"{' '.join(map(str, ins))} -> {' '.join(map(str, outs))}\n")
    out.close()


def shot(m, work, rnd):
    harry = face_tables(m)
    sp = m.alloc(0xA2)
    out = open(f"{work}/diff/shot.txt", "w")
    out.write("# SetupPlayerShotSprite + frames x HandlePlayerShotSprite: kind posV posH speedV speedH dir frames "
              "-> per frame (after setup, then each frame) posV:posH:atV:atH:fixH:fixV:dir:timer:face:flag8b, "
              "then hotRect(t:l:b:r) layer\n")
    for case in range(CASES // 4):
        kind = rnd.choice([0, 1, 4, 10, rnd.randint(0, 12)])
        pv, ph = rnd.randint(-10, 4100), rnd.randint(-10, 4100)
        sv, sh = rnd.randint(-3000, 3000), rnd.randint(-3000, 3000)
        d = rnd.choice([0, rnd.randint(0, 35)])
        frames = rnd.randint(0, 8)
        m.write(sp, bytes([0x77]) * 0xA2)
        m.w16(sp, kind)
        m.w16(sp + 2, pv)
        m.w16(sp + 4, ph)
        m.call("SetupPlayerShotSprite", sp)
        # MakeNewShot / HandleKeys then set the speed and direction
        m.w16(sp + 0x4E, sv)
        m.w16(sp + 0x50, sh)
        m.w16(sp + 0x66, d)
        states = []
        for f in range(frames + 1):
            if f:
                m.call("HandlePlayerShotSprite", sp)
            states.append("%d:%d:%d:%d:%d:%d:%d:%d:%s:%d" % (
                m.r16(sp + 2), m.r16(sp + 4), m.r16(sp + 0x5A), m.r16(sp + 0x5C),
                struct.unpack(">i", m.read(sp + 0x5E, 4))[0], struct.unpack(">i", m.read(sp + 0x62, 4))[0],
                m.r16(sp + 0x66), m.r16(sp + 0x74), face_name(m, m.r32(sp + 0x7E), harry), m.read(sp + 0x8B, 1)[0]))
        hot = ":".join(map(str, struct.unpack(">hhhh", m.read(sp + 6, 8))))
        out.write(f"{kind} {pv} {ph} {sv} {sh} {d} {frames} -> {','.join(states)} {hot} {m.r16(sp + 0x4C)}\n")
    out.close()


def shot_tiles(m, work, rnd):
    from emu_mask import masks
    ms = masks(work)
    lines = open(f"{work}/diff/facemasks.txt").read().splitlines()
    k = next(i for i, ln in enumerate(lines) if ln.startswith("= 4021 "))
    shot_mask = {(x, y) for y in range(8) for x in range(8) if lines[k + 1 + y][x] == "#"}
    m.call("InitTileHotRects")
    ev = []
    m.stub_routine("STPlayRegSound", lambda mm: ev.append("p:%d" % mm.get(3)) or 0)
    m.stub_routine("STPlay3DSound", lambda mm: ev.append("3d:%d" % mm.get(3)) or 0)
    m.stub_routine("EasyExplosion", lambda mm: ev.append("x:%d:%d:%d:%d" % tuple(s16(mm.get(i)) for i in (3, 4, 7, 8))) or 0)
    m.stub_routine("AddFGRedraw", 0)
    for slot, snd in ((0x10065C08, 500), (0x10065BB0, 4700)):
        m.w32(m.glob(slot), snd)
    sat = m.glob(0x10065B70)
    walls = m.alloc(64 * 0x2E)
    m.w32(m.glob(0x100658D4), walls)
    shot_face = m.alloc(0x1C)
    m.w32(m.glob(0x1006613C), shot_face)
    sp, face = m.alloc(0xA2), m.alloc(0x1C)
    out = open(f"{work}/diff/shottiles.txt", "w")
    out.write("# CheckTileCollisions + HitPlayerShotTileSprite: level kind atV atH speedV speedH timer faceB faceR "
              "-> posV posH atV atH fixH fixV speedV speedH frozen flag8b events\n")
    n = 0
    for lvl in range(1, 16):
        data = open(f"{work}/res/Episode_1/Hlvl_{lvl}.bin", "rb").read()
        m.load_level(data)
        mark = m.heap
        grid = lambda x, y: struct.unpack(">H", data[0x8000 + (x * 128 + y) * 2:0x8000 + (x * 128 + y) * 2 + 2])[0]
        walls_at = [(x, y) for x in range(1, 127) for y in range(1, 127) if grid(x, y) != 0xFFFF
                    and any(grid(a, b) == 0xFFFF for a, b in ((x, y - 1), (x + 1, y), (x - 1, y), (x, y + 1)))]
        for _ in range(1500):
            m.heap = mark
            m.regions.clear()
            for w in range(32):
                m.w32(walls + 0x1A + w * 0x2E, m.make_region(ms[f"wall{w}"]))
            m.w32(shot_face + 0x14, m.make_region(shot_mask))
            x, y = rnd.choice(walls_at) if walls_at and rnd.random() < 0.9 else (rnd.randint(1, 126), rnd.randint(1, 126))
            kind = rnd.choice([0, 0, 1, 4, 4, 10])
            av, ah = y * 32 + rnd.randint(-40, 40), x * 32 + rnd.randint(-40, 40)
            sv, sh = rnd.randint(-3000, 3000), rnd.randint(-3000, 3000)
            timer = rnd.choice([0, 0, 0, 1, 3])
            fb, fr = (64, 64) if kind == 10 else rnd.choice([(8, 8), (12, 12)])
            m.write(sp, bytes(0xA2))
            m.w16(sp, kind)
            m.w16(sp + 2, av)
            m.w16(sp + 4, ah)
            m.call("SetupPlayerShotSprite", sp)
            m.w16(sp + 0x4E, sv)
            m.w16(sp + 0x50, sh)
            m.w16(sp + 0x74, timer)
            m.write(sp + 0x8B, b"\0")
            m.write(face, struct.pack(">hhhh", 0, 0, fb, fr) + bytes(0x14))
            m.w32(sp + 0x7E, face)
            m.w32(sat + 0x78, sp)
            m.w32(sp + 0x2E, 0)
            ev.clear()
            m.call("CheckTileCollisions")
            outs = [m.r16(sp + 2), m.r16(sp + 4), m.r16(sp + 0x5A), m.r16(sp + 0x5C),
                    struct.unpack(">i", m.read(sp + 0x5E, 4))[0], struct.unpack(">i", m.read(sp + 0x62, 4))[0],
                    m.r16(sp + 0x4E), m.r16(sp + 0x50), m.read(sp + 0x92, 1)[0], m.read(sp + 0x8B, 1)[0],
                    ",".join(ev) or "-"]
            out.write(f"{lvl} {kind} {av} {ah} {sv} {sh} {timer} {fb} {fr} -> {' '.join(map(str, outs))}\n")
            n += 1
    out.close()
    print(f"shottiles: {n} cases")


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    rnd = random.Random("shots")
    weapon(Machine(f"{work}/ppc"), work, rnd)
    shot(Machine(f"{work}/ppc"), work, rnd)
    shot_tiles(Machine(f"{work}/ppc"), work, rnd)
    print("weapon, shot written")


if __name__ == "__main__":
    main()
