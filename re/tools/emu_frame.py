"""Record HandlePlayerSprite - Harry's whole frame except the keys
(HandleKeys is stubbed; it gets its own test) - on the real levels:
comfort delta, all timers, idle chatter, the inventory check, MoveChair,
HandlePlayerGlide, HandleHalfPipe, JumpScale, wetness, max speed, the
position update, the shake and death.

Each line: "<inputs> -> <outputs>"; the fields are named in the first
line (# ...). Usage: python3 -I emu_frame.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from emu_air import cells, AIR_STATES  # noqa: E402

# word globals reached through TOC slots, in the order HandlePlayerSprite
# counts the timers down; then the other move / state words
TIMERS = [("scoot", 0x10065E84), ("kick", 0x10065E80), ("inch", 0x10065E7C), ("invswitch", 0x10065E4C),
          ("fire", 0x10065E48), ("ouch", 0x10065E44), ("t40", 0x10065E40), ("itemuse", 0x10065E3C),
          ("recoil", 0x10065E38), ("caffeine", 0x10065AD4), ("cheat1", 0x10065E34), ("cheat2", 0x10065E30),
          ("wavy", 0x10065E2C), ("t28", 0x10065E28), ("t24", 0x10065E24), ("chatter", 0x10065AC8),
          ("shake", 0x10065E68)]
WORDS = [("pose", 0x10065E78), ("comfortdelta", 0x10065E6C), ("wet", 0x10065E8C), ("contact", 0x10065E1C),
         ("power", 0x10065E0C), ("glide", 0x10065E60), ("flag5c", 0x10065E5C)]
BYTES = [("kickreset", 0x10065E20), ("flag50", 0x10065E50), ("bgcontact", 0x10065E54), ("wallcontact", 0x10065E58)]
DIRECT = [("impulse", 0x10068A96), ("comfort", 0x1006856A), ("comfortshown", 0x10068570),
          ("favorshown", 0x10068574), ("floor", 0x10068578), ("curslot", 0x1006857A), ("surface", 0x1006856C)]
OUT_ONLY = [("playerdir", 0x10065E90), ("playerh", 0x100659B4), ("playerv", 0x100659B0)]
INVENTORY, SPEED_COPY, FACES, PLAYER, PHYSICS = 0x10065AC4, 0x10065E18, 0x10065E9C, 0x10065980, 0x100659D8
TV_DEATH = 0x10066FAC
SPRITE = [("posV", 0x02), ("posH", 0x04), ("atV", 0x5A), ("atH", 0x5C), ("speedV", 0x4E), ("speedH", 0x50),
          ("dir", 0x66), ("air", 0x8E), ("slide", 0x9C), ("flip", 0x90), ("scale0", 0x52), ("scale1", 0x54)]
CASES = 20000


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    keys, sounds = [], []
    m.stub_routine("HandleKeys", 0, keys)
    m.stub_routine("STPlayRegSound", 0, sounds)
    m.stub_routine(0x10056DE8, 0)      # sound playing? no
    m.stub_routine(0x10056C0C, 0)      # stop a sound
    for f in ("GammaFadeOut", "GammaFadeIn", 0x1002549C):
        m.stub_routine(f, 0)
    g = {name: m.glob(slot) for name, slot in TIMERS + WORDS + BYTES + OUT_ONLY}
    inv, speed_copy, player = m.glob(INVENTORY), m.glob(SPEED_COPY), m.glob(PLAYER)
    faces = m.alloc(144 * 0x1C)
    m.w32(m.glob(FACES), faces)
    for i, v in enumerate((50, 70, 245, 252, 150)):
        m.w16(m.glob(PHYSICS) + 2 * i, v)
    sp = m.alloc(0xA2)
    rnd = random.Random("frame")
    names_in = ([n for n, _ in TIMERS + WORDS + BYTES + DIRECT] + [n for n, _ in SPRITE] +
                ["fixH", "fixV", "frozen", "inventory0", "inventorycur", "seed", "level"])
    names_out = ([n for n, _ in TIMERS + WORDS + BYTES + DIRECT + OUT_ONLY] + [n for n, _ in SPRITE] +
                 ["fixH", "fixV", "face", "hotRect(t l b r)", "dead", "speedcopyV", "speedcopyH", "keys", "sounds"])
    out = open(f"{work}/diff/frame.txt", "w")
    out.write("# HandlePlayerSprite (HandleKeys stubbed): " + " ".join(names_in) + " -> " + " ".join(names_out) + "\n")
    per = CASES // 15
    for lvl in range(1, 16):
        data = open(f"{work}/res/Episode_1/Hlvl_{lvl}.bin", "rb").read()
        m.load_level(data)
        special = cells(data, set(range(1, 16)))
        for _ in range(per):
            st = {}
            for n, _ in TIMERS:
                st[n] = rnd.choice([0, 0, 1, rnd.randint(1, 30)])
            st["chatter"] = rnd.choice([0, 1, 1, rnd.randint(2, 300)])
            st["caffeine"] = rnd.choice([0, 0, 1, rnd.randint(1, 270)])
            st["pose"] = rnd.choice([0, 36, 72, 108])
            st["comfortdelta"] = rnd.choice([0, 0, rnd.randint(-20, 20)])
            st["wet"] = rnd.choice([0, 0, rnd.randint(0, 200)])
            st["contact"] = rnd.choice([0, 0, 64, 128, 256])
            st["power"] = rnd.choice([0, 128, 256])
            st["glide"] = rnd.choice([0, 0, rnd.randint(0, 120)])
            st["flag5c"] = rnd.choice([0, 1])
            for n, _ in BYTES:
                st[n] = rnd.choice([0, 1])
            st["impulse"] = rnd.choice([0, rnd.randint(-300, 300)])
            st["comfort"] = rnd.randint(-5, 150)
            st["comfortshown"] = rnd.choice([rnd.randint(0, 144), 0, 1])
            st["favorshown"] = rnd.choice([rnd.randint(0, 144), 0, 1])
            st["floor"] = rnd.randint(0, 4)
            st["curslot"] = rnd.choice([-1, 0, rnd.randint(0, 11)])
            st["surface"] = rnd.randint(0, 144)
            x, y = rnd.choice(special) if special and rnd.random() < 0.7 else (rnd.randint(2, 125), rnd.randint(2, 125))
            ah, av = x * 32 - 32 + rnd.randint(0, 31), y * 32 - 32 + rnd.randint(0, 31)
            spr = {"posV": av, "posH": ah, "atV": av, "atH": ah,
                   "speedV": rnd.choice([0, rnd.randint(-3000, 3000), rnd.randint(-300, 300)]),
                   "speedH": rnd.choice([0, rnd.randint(-3000, 3000), rnd.randint(-300, 300)]),
                   "dir": rnd.choice([rnd.randint(0, 35), -1, 36]), "air": rnd.choice(AIR_STATES),
                   "slide": rnd.choice([0, rnd.randint(0, 15)]), "flip": rnd.choice([0, 0x26AD, 0x26B0]),
                   "scale0": 0x100, "scale1": 0x100}
            fh = rnd.choice([(ah << 8) | rnd.randint(0, 255), 0, -5, 0x100001])
            fv = rnd.choice([(av << 8) | rnd.randint(0, 255), (av << 8) | rnd.randint(0, 255), 0])
            frozen = 1 if rnd.random() < 0.03 else 0
            inv0 = rnd.choice([-1, 11299, 11599])
            invcur = rnd.choice([-1, 11799])
            seed = rnd.randint(1, 0x7FFFFFFE)
            # write everything
            for n, _ in TIMERS + WORDS:
                m.w16(g[n], st[n])
            for n, _ in BYTES:
                m.write(g[n], bytes([st[n]]))
            for n, a in DIRECT:
                m.w16(a, st[n])
            for n, _ in OUT_ONLY:
                m.w16(g[n], 0x4444)
            m.w16(speed_copy, 0x4444)
            m.w16(speed_copy + 2, 0x4444)
            for i in range(12):
                m.w16(inv + 4 * i, -1)
                m.w16(inv + 4 * i + 2, 0)
            m.w16(inv, inv0)
            if st["curslot"] > 0:
                m.w16(inv + 4 * st["curslot"], invcur)
            m.write(sp, bytes(0xA2))
            for n, off in SPRITE:
                m.w16(sp + off, spr[n])
            m.w32(sp + 0x5E, fh)
            m.w32(sp + 0x62, fv)
            m.write(sp + 0x92, bytes([frozen]))
            m.w32(sp + 0x1A, 0x10066FC4)        # task = HandlePlayerSprite
            m.w32(player, sp)
            m.rand_seed = seed
            keys.clear()
            sounds.clear()
            m.call("HandlePlayerSprite", sp, regs={30: 0x10068AA4})
            ins = ([st[n] for n, _ in TIMERS + WORDS + BYTES + DIRECT] + [spr[n] for n, _ in SPRITE] +
                   [fh, fv, frozen, inv0, invcur, seed, lvl])
            outs = ([m.r16(g[n]) for n, _ in TIMERS + WORDS] + [m.read(g[n], 1)[0] for n, _ in BYTES] +
                    [m.r16(a) for _, a in DIRECT] + [m.r16(g[n]) for n, _ in OUT_ONLY] +
                    [m.r16(sp + off) for _, off in SPRITE] +
                    [struct.unpack(">i", m.read(sp + 0x5E, 4))[0], struct.unpack(">i", m.read(sp + 0x62, 4))[0],
                     (m.r32(sp + 0x7E) - faces) // 0x1C if m.r32(sp + 0x7E) else -999] +
                    list(struct.unpack(">hhhh", m.read(sp + 0x06, 8))) +
                    [int(m.r32(sp + 0x1A) == TV_DEATH), m.r16(speed_copy), m.r16(speed_copy + 2), len(keys), len(sounds)])
            out.write(f"{' '.join(map(str, ins))} -> {' '.join(map(str, outs))}\n")
    out.close()
    print(f"frame: {per * 15} cases")


if __name__ == "__main__":
    main()
