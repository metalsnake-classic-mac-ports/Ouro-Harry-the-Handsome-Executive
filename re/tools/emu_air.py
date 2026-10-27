"""Record HandleHalfPipe and JumpScale (half pipes, jumps, falling into
holes) of the original on the real levels.

Lines:  halfpipe: level player atV atH fixH fixV speedH speedV air slide flip
            -> speedH speedV air flip lastH lastV slide sounds
        jump: level seed atV atH fixH fixV speedH speedV dir air flip comfort
            -> air scale0 scale1 speedH speedV dir fixH fixV posV posH atV atH
               comfort sounds viewV viewH
Usage: python3 -I emu_air.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402

PLAYER_SLOT = 0x10065980          # *slot = the player's sprite
VIEW_SLOT = 0x100659BC            # the view position (v, h) JumpScale recentres
COMFORT = 0x1006856A
AIR_STATES = [0, 0, 0] + list(range(1, 36)) + list(range(213, 223)) + list(range(998, 1047))


def cells(data, kinds):
    bgk = [(data[0x12094 + i // 2] >> (4 if i % 2 == 0 else 0)) & 15 for i in range(96)]
    out = []
    for x in range(1, 127):
        for y in range(1, 127):
            b = struct.unpack(">H", data[(x * 128 + y) * 2:(x * 128 + y) * 2 + 2])[0]
            if b < 96 and bgk[b] in kinds:
                out.append((x, y))
    return out


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    sounds = []
    m.stub_routine("STPlayRegSound", 0, sounds)
    for f in ("GammaFadeOut", "GammaFadeIn", 0x1002549C):
        m.stub_routine(f, 0)
    sp, other = m.alloc(0xA2), m.alloc(0xA2)
    player = m.glob(PLAYER_SLOT)
    view = m.glob(VIEW_SLOT)
    rnd = random.Random("air")
    hp = open(f"{work}/diff/halfpipe.txt", "w")
    hp.write("# HandleHalfPipe: level player atV atH fixH fixV speedH speedV air slide flip -> "
             "speedH speedV air flip lastH lastV slide sounds\n")
    js = open(f"{work}/diff/jump.txt", "w")
    js.write("# JumpScale: level seed atV atH fixH fixV speedH speedV dir air flip comfort -> air scale0 scale1 "
             "speedH speedV dir fixH fixV posV posH atV atH comfort sounds viewV viewH\n")
    for lvl in range(1, 16):
        data = open(f"{work}/res/Episode_1/Hlvl_{lvl}.bin", "rb").read()
        m.load_level(data)
        special = cells(data, set(range(7, 16)))
        # half pipes are in levels 7, 12, 14, holes in 11 and 15
        for _ in range(4000 if special else 300):
            x, y = rnd.choice(special) if special and rnd.random() < 0.8 else (rnd.randint(1, 126), rnd.randint(1, 126))
            # the sprite's top-left so that its centre (+32) lies in that tile
            ah, av = x * 32 - 32 + rnd.randint(0, 31), y * 32 - 32 + rnd.randint(0, 31)
            fh, fv = (ah << 8) | rnd.randint(0, 255), (av << 8) | rnd.randint(0, 255)
            sh = rnd.choice([rnd.randint(-2500, 2500), rnd.randint(-600, 600), 0])
            sv = rnd.choice([rnd.randint(-2500, 2500), rnd.randint(-600, 600), 0])
            air = rnd.choice(AIR_STATES)
            slide = rnd.choice([0, 0, rnd.randint(0, 15)])
            flip = rnd.choice([0, 0x26AD, 0x26AE, 0x26AF, 0x26B0])
            is_player = rnd.random() < 0.8
            s = sp if is_player else other
            m.w32(player, sp)
            m.write(s, bytes(0xA2))
            for off, v in ((0x02, av), (0x04, ah), (0x5A, av), (0x5C, ah), (0x50, sh), (0x4E, sv),
                           (0x8E, air), (0x9C, slide), (0x90, flip)):
                m.w16(s + off, v)
            m.w32(s + 0x5E, fh)
            m.w32(s + 0x62, fv)
            sounds.clear()
            m.call("HandleHalfPipe", s, 0x20, 0x1234)
            hp.write(f"{lvl} {int(is_player)} {av} {ah} {fh} {fv} {sh} {sv} {air} {slide} {flip} -> "
                     f"{m.r16(s + 0x50)} {m.r16(s + 0x4E)} {m.r16(s + 0x8E)} {m.r16(s + 0x90)} "
                     f"{m.r16(s + 0x98)} {m.r16(s + 0x9A)} {m.r16(s + 0x9C)} {len(sounds)}\n")
            # JumpScale (always the player, as HandlePlayerSprite calls it)
            d = rnd.randint(0, 35)
            comfort = rnd.randint(0, 144)
            seed = rnd.randint(1, 0x7FFFFFFE)
            m.rand_seed = seed
            m.write(sp, bytes(0xA2))
            for off, v in ((0x02, av), (0x04, ah), (0x5A, av), (0x5C, ah), (0x50, sh), (0x4E, sv),
                           (0x66, d), (0x8E, air), (0x90, flip), (0x52, 0x100), (0x54, 0x100)):
                m.w16(sp + off, v)
            m.w32(sp + 0x5E, fh)
            m.w32(sp + 0x62, fv)
            m.w16(COMFORT, comfort)
            m.w16(view, 0x5555)
            m.w16(view + 2, 0x5555)
            sounds.clear()
            # r30 as HandlePlayerSprite leaves it: JumpScale uses it unset
            # when no case matches (air 999)
            m.call("JumpScale", sp, 0x20, 1, 2, 3, 1, regs={30: 0x10068AA4})
            outs = [m.r16(sp + 0x8E), m.r16(sp + 0x52), m.r16(sp + 0x54), m.r16(sp + 0x50), m.r16(sp + 0x4E),
                    m.r16(sp + 0x66), struct.unpack(">i", m.read(sp + 0x5E, 4))[0],
                    struct.unpack(">i", m.read(sp + 0x62, 4))[0], m.r16(sp + 0x02), m.r16(sp + 0x04),
                    m.r16(sp + 0x5A), m.r16(sp + 0x5C), m.r16(COMFORT), len(sounds), m.r16(view), m.r16(view + 2)]
            js.write(f"{lvl} {seed} {av} {ah} {fh} {fv} {sh} {sv} {d} {air} {flip} {comfort} -> "
                     f"{' '.join(map(str, outs))}\n")
    print("halfpipe, jump written")


if __name__ == "__main__":
    main()
