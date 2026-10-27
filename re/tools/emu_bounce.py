"""Record WallBounce of the original for differential tests.

The pixel-exact mask test (FUN_10054124, regions) is stubbed: its result
is an input of the case, and it does not move the sprite. One line per
case: kind first tileV tileH centerV centerH posV posH speedH speedV
bounce mask -> return centerV centerH posV posH speedH speedV fixH fixV
calls(mask). hot rect (22, 22, 42, 42), half size 32 as the player uses.

Usage: python3 -I emu_bounce.py [re/work/ppc] [out dir]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402

POS_V, POS_H, CEN_V, CEN_H, FIX_H, FIX_V, SPEED_V, SPEED_H = 0x02, 0x04, 0x5A, 0x5C, 0x5E, 0x62, 0x4E, 0x50
HOT = (22, 22, 42, 42)
CASES = 12000


def main():
    ppc = sys.argv[1] if len(sys.argv) > 1 else "re/work/ppc"
    out_dir = sys.argv[2] if len(sys.argv) > 2 else "re/work/diff"
    m = Machine(ppc)
    rnd = random.Random("wallbounce")
    state = {"mask": 0}
    calls = []
    m.stub_routine(0x10054124, lambda mm: state["mask"], calls)
    # tile mask faces (64 x 0x2e bytes, *_DAT_100658d4): only passed on to
    # the stubbed mask test, so a zeroed table will do
    m.w32(m.glob(0x100658D4), m.alloc(64 * 0x2E))
    sp = m.alloc(0xA2)
    tile = m.alloc(4)
    hot = m.alloc(8)
    m.write(hot, struct.pack(">hhhh", *HOT))
    with open(os.path.join(out_dir, "wallbounce.txt"), "w") as out:
        out.write("# WallBounce (PPC 0x10019dac), mask test stubbed; " + __doc__.splitlines()[3].strip() + "\n")
        for k in range(CASES):
            kind = k % 34 if k % 50 else rnd.choice([-1, 40, 56, 60])
            first = rnd.randint(0, 1)
            tv, th = rnd.randint(10, 100) * 32, rnd.randint(10, 100) * 32
            cv, ch = tv - 32 + rnd.randint(-40, 40), th - 32 + rnd.randint(-40, 40)
            pv, ph = cv + rnd.choice([0, 0, rnd.randint(-5, 5)]), ch + rnd.choice([0, 0, rnd.randint(-5, 5)])
            sh, sv = rnd.choice([rnd.randint(-2500, 2500), rnd.randint(-30, 30), 0]), rnd.choice([rnd.randint(-2500, 2500), rnd.randint(-30, 30), 0])
            bounce = rnd.choice([150, 150, rnd.randint(0, 300)])
            state["mask"] = rnd.randint(0, 1)
            for off, v in ((POS_V, pv), (POS_H, ph), (CEN_V, cv), (CEN_H, ch), (SPEED_H, sh), (SPEED_V, sv)):
                m.w16(sp + off, v)
            m.w32(sp + FIX_H, 0x7777)
            m.w32(sp + FIX_V, 0x7777)
            m.write(tile, struct.pack(">hh", tv, th))
            calls.clear()
            r = m.call("WallBounce", sp, kind & 0xFFFF, tile, 32, bounce, hot, 0x1234, first)
            r = (r & 0xFF) if (r & 0xFF) < 0x80 else (r & 0xFF) - 0x100
            outs = [r, m.r16(sp + CEN_V), m.r16(sp + CEN_H), m.r16(sp + POS_V), m.r16(sp + POS_H),
                    m.r16(sp + SPEED_H), m.r16(sp + SPEED_V),
                    struct.unpack(">i", m.read(sp + FIX_H, 4))[0], struct.unpack(">i", m.read(sp + FIX_V, 4))[0], len(calls)]
            ins = [kind, first, tv, th, cv, ch, pv, ph, sh, sv, bounce, state["mask"]]
            out.write(f"{' '.join(map(str, ins))} -> {' '.join(map(str, outs))}\n")
    print(f"wallbounce: {CASES} cases")

    # RectBounceFake (PPC 0x1001e954): rect obstacle, no mask test
    rect = m.alloc(8)
    rnd = random.Random("rectbounce")
    with open(os.path.join(out_dir, "rectbounce.txt"), "w") as out:
        out.write("# RectBounceFake: tileV tileH rect(t l b r) atV atH speedH speedV bounce -> "
                  "atV atH speedH speedV fixH fixV\n")
        for _ in range(CASES):
            tv, th = rnd.randint(10, 100) * 32, rnd.randint(10, 100) * 32
            r = rnd.choice([(2, 2, 30, 30), (6, 6, 26, 26), (0, 0, 32, 32),
                            tuple(sorted(rnd.sample(range(0, 33), 2)))[:1] + tuple(sorted(rnd.sample(range(0, 33), 2)))[:1]
                            + (rnd.randint(17, 32), rnd.randint(17, 32))])
            av, ah = tv - 32 + rnd.randint(-36, 36), th - 32 + rnd.randint(-36, 36)
            if rnd.random() < 0.25:
                # exactly diagonal to the rect's centre (ties of the axis choice)
                mv, mh = tv + r[0] + ((r[2] - r[0]) >> 1), th + r[1] + ((r[3] - r[1]) >> 1)
                d = rnd.randint(0, 20)
                av, ah = mv - 32 + rnd.choice([-d, d]), mh - 32 + rnd.choice([-d, d])
            sh, sv = rnd.choice([rnd.randint(-2500, 2500), rnd.randint(-30, 30), 0]), rnd.choice([rnd.randint(-2500, 2500), rnd.randint(-30, 30), 0])
            bounce = rnd.choice([150, 150, rnd.randint(0, 300)])
            for off, v in ((CEN_V, av), (CEN_H, ah), (SPEED_H, sh), (SPEED_V, sv)):
                m.w16(sp + off, v)
            m.w32(sp + FIX_H, 0x7777)
            m.w32(sp + FIX_V, 0x7777)
            m.write(tile, struct.pack(">hh", tv, th))
            m.write(rect, struct.pack(">hhhh", *r))
            m.call("RectBounceFake", sp, tile, rect, 32, bounce, hot)
            outs = [m.r16(sp + CEN_V), m.r16(sp + CEN_H), m.r16(sp + SPEED_H), m.r16(sp + SPEED_V),
                    struct.unpack(">i", m.read(sp + FIX_H, 4))[0], struct.unpack(">i", m.read(sp + FIX_V, 4))[0]]
            ins = [tv, th, *r, av, ah, sh, sv, bounce]
            out.write(f"{' '.join(map(str, ins))} -> {' '.join(map(str, outs))}\n")
    print(f"rectbounce: {CASES} cases")


if __name__ == "__main__":
    main()


def bg_cases(m, rnd, work="re/work", per=CASES // 15):
    """Yield (four neighbouring foreground kinds, (tileV, tileH), m) at
    background obstacles and free tiles next to walls of levels 1-15,
    with each level loaded into the machine."""
    for lvl in range(1, 16):
        data = open(os.path.join(work, "res/Episode_1", f"Hlvl_{lvl}.bin"), "rb").read()
        m.load_level(data)
        bgk = [(data[0x12094 + i // 2] >> (4 if i % 2 == 0 else 0)) & 15 for i in range(96)]
        grid = lambda base, x, y: struct.unpack(">H", data[base + (x * 128 + y) * 2:base + (x * 128 + y) * 2 + 2])[0]
        obstacles, edges = [], []
        for x in range(1, 127):
            for y in range(1, 127):
                b = grid(0, x, y)
                if b < 96 and bgk[b] == 1:
                    obstacles.append((x, y))
                elif grid(0x8000, x, y) == 0xFFFF and any(
                        grid(0x8000, a, c) < 8 for a, c in ((x, y - 1), (x + 1, y), (x - 1, y), (x, y + 1))):
                    edges.append((x, y))
        for _ in range(per):
            pool = obstacles if obstacles and rnd.random() < 0.5 else edges
            x, y = rnd.choice(pool) if pool else (rnd.randint(1, 126), rnd.randint(1, 126))
            kinds = [m.call("LookupFGTileKind", m.call("GetFGTile", a, b) & 0xFFFF)
                     for a, b in ((x, y - 1), (x + 1, y), (x - 1, y), (x, y + 1))]
            kinds = [k - 0x10000 if k & 0x8000 else k & 0xFFFF for k in kinds]
            yield kinds, (y * 32, x * 32), m


def bg_tile_bounce(ppc, out_dir, res_dir="re/work/res/Episode_1"):
    """BGTileBounce (PPC 0x1001c8a4) on real levels: tile positions all over
    levels 1-15, the four neighbouring foreground kinds (as the original
    looks them up) recorded with each case; mask test stubbed."""
    m = Machine(ppc)
    rnd = random.Random("bgbounce")
    state = {"mask": 0}
    calls = []
    m.stub_routine(0x10054124, lambda mm: state["mask"], calls)
    m.w32(m.glob(0x100658D0), m.alloc(0x40))
    sp = m.alloc(0xA2)
    tile = m.alloc(4)
    hot = m.alloc(8)
    m.write(hot, struct.pack(">hhhh", *HOT))
    with open(os.path.join(out_dir, "bgbounce.txt"), "w") as out:
        out.write("# BGTileBounce: kindUp kindRight kindLeft kindDown first tileV tileH atV atH speedH speedV "
                  "bounce mask -> return atV atH speedH speedV fixH fixV calls\n")
        for kinds, (tv, th), _ in bg_cases(m, rnd):
            av, ah = tv - 32 + rnd.randint(-30, 30), th - 32 + rnd.randint(-30, 30)
            sh, sv = rnd.choice([rnd.randint(-2500, 2500), rnd.randint(-30, 30)]), rnd.choice([rnd.randint(-2500, 2500), rnd.randint(-30, 30)])
            bounce = rnd.choice([150, 150, rnd.randint(0, 300)])
            first = rnd.randint(0, 1)
            state["mask"] = rnd.choice([1, 1, 1, 0])
            for off, v in ((CEN_V, av), (CEN_H, ah), (POS_V, av), (POS_H, ah), (SPEED_H, sh), (SPEED_V, sv)):
                m.w16(sp + off, v)
            m.w32(sp + FIX_H, 0x7777)
            m.w32(sp + FIX_V, 0x7777)
            m.write(tile, struct.pack(">hh", tv, th))
            calls.clear()
            r = m.call("BGTileBounce", sp, 1, tile, 32, bounce, hot, 0x1234, first)
            outs = [r & 0xFF, m.r16(sp + CEN_V), m.r16(sp + CEN_H), m.r16(sp + SPEED_H), m.r16(sp + SPEED_V),
                    struct.unpack(">i", m.read(sp + FIX_H, 4))[0], struct.unpack(">i", m.read(sp + FIX_V, 4))[0], len(calls)]
            ins = kinds + [first, tv, th, av, ah, sh, sv, bounce, state["mask"]]
            out.write(f"{' '.join(map(str, ins))} -> {' '.join(map(str, outs))}\n")
    print("bgbounce written")


if __name__ == "__main__":
    bg_tile_bounce(sys.argv[1] if len(sys.argv) > 1 else "re/work/ppc", sys.argv[2] if len(sys.argv) > 2 else "re/work/diff")
