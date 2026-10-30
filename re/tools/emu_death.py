"""Record Harry's death (HandlePlayerSpriteDeath): a player record with
its timer, direction and position at random, comfort / favor shown at
random around 0, then frames of the task. SATNewSprite and EasyExplosion
are stubbed (zeroed records); what the task sets on them is logged.

Each line: "<name=value ...> -> <dump> | <globals> events=<per frame, ';'>"
  events  p:<snd>, fade, x:<h>:<v>:<kind>:<dir>:<+0x8b>,
          new:<kind>:<h>:<v>:<f6e>:<f7c>:<+0x8b>
Usage: python3 -I emu_death.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from sprites import Faces, tvectors, dump  # noqa: E402

# TOC slots: the shake, the recoil, the "player off" flag that ends the loop
SHAKE, RECOIL_SLOT, DONE = 0x10065AD0, 0x10065E38, 0x10065ACC
COMFORT_SHOWN, FAVOR_SHOWN, SHOT_KIND, CUR_SLOT = 0x10068570, 0x10068574, 0x10068576, 0x1006857A
SOUNDS = {0x10065B40: 4901, 0x10065B44: 4900, 0x10065BF4: 602}
RUNS = 3000


def s16(x):
    x &= 0xFFFF
    return x - 0x10000 if x & 0x8000 else x


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m, work)
    m.call(0x1005FCE8, 0, 1, 1, 0x40)               # SATConfigure as InitSAT does
    m.call(0x1005FD08, 0xA2)                        # the record size
    m.write(m.glob(0x10066254), b"\1")              # SAT's (graphics) init done
    m.stub_routine(0x1005F340, lambda mm: mm.alloc(0x40))   # mask faces: unused here
    m.call("InitPlayerSprite")
    tv = tvectors(f"{work}/ppc")
    ev, made = [], []
    m.stub_routine("STPlayRegSound", lambda mm: ev.append("p:%d" % mm.get(3)) or 0)
    m.stub_routine("FadeMusic", lambda mm: ev.append("fade") or 0)

    def explosion(mm):
        rec = mm.alloc(0xA2)
        made.append(("x", rec, "x:%d:%d:%d:%d" % tuple(s16(mm.get(i)) for i in (3, 4, 7, 8))))
        ev.append(rec)
        return rec
    m.stub_routine("EasyExplosion", explosion)

    def new_sprite(mm):
        rec = mm.alloc(0xA2)
        made.append(("n", rec, "new:%d:%d:%d" % tuple(s16(mm.get(i)) for i in (3, 4, 5))))
        ev.append(rec)
        return rec
    m.stub_routine("SATNewSprite", new_sprite)
    for slot, snd in SOUNDS.items():
        m.w32(m.glob(slot), snd)
    names = {v: k for k, v in tv.items()}

    def frame_events():
        out = []
        recs = {r: (t, s) for t, r, s in made}
        for e in ev:
            if not isinstance(e, int):
                out.append(e)
                continue
            t, s = recs[e]
            if t == "x":
                out.append(f"{s}:{m.read(e + 0x8B, 1)[0]}")
            else:
                f6e = struct.unpack(">i", m.read(e + 0x6E, 4))[0]
                out.append(f"{s}:{f6e}:{s16(m.r16(e + 0x7C))}:{m.read(e + 0x8B, 1)[0]}")
        return ",".join(out) or "-"

    sp = m.alloc(0xA2)
    mark = m.heap
    rnd = random.Random("death")
    fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())
    out = open(f"{work}/diff/death.txt", "w")
    out.write("# death: name=value inputs -> dump | globals events\n")
    for _ in range(RUNS):
        m.heap = mark
        st = {"timer": rnd.choice([0, 0, 0, rnd.randint(0, 120), 17, 18, 44, 45, 51, 52, 109, 110]),
              "dir": rnd.randint(0, 35),
              "h": rnd.randint(-100, 4000), "v": rnd.randint(-100, 4000),
              "comfort": rnd.choice([0, 0, 1, -3, 50]), "favor": rnd.choice([0, 1, 1, 60, -2]),
              "slot": rnd.choice([-1, 0, 3]), "shot": rnd.choice([0, 2, 4]), "recoil": rnd.choice([0, 3]),
              "frames": rnd.choice([1, 2, rnd.randint(1, 130)])}
        m.write(sp, bytes(0xA2))
        m.w16(sp, 69)
        m.w16(sp + 2, st["v"])
        m.w16(sp + 4, st["h"])
        m.call("SetupPlayerSprite", sp)
        m.w32(sp + 0x1A, names["HandlePlayerSpriteDeath"])
        m.w16(sp + 0x74, st["timer"])
        m.w16(sp + 0x66, st["dir"])
        m.w16(sp + 0x5A, st["v"])
        m.w16(sp + 0x5C, st["h"])
        m.w16(COMFORT_SHOWN, st["comfort"])
        m.w16(FAVOR_SHOWN, st["favor"])
        m.w16(CUR_SLOT, st["slot"])
        m.w16(SHOT_KIND, st["shot"])
        m.w16(m.glob(RECOIL_SLOT), st["recoil"])
        m.w16(m.glob(SHAKE), 0)
        m.write(m.glob(DONE), b"\0")
        evs = []
        for _f in range(st["frames"]):
            ev.clear()
            made.clear()
            m.call("HandlePlayerSpriteDeath", sp)
            evs.append(frame_events())
        o = (f"slot={s16(m.r16(CUR_SLOT))} shot={s16(m.r16(SHOT_KIND))} recoil={s16(m.r16(m.glob(RECOIL_SLOT)))} "
             f"shake={s16(m.r16(m.glob(SHAKE)))} done={m.read(m.glob(DONE), 1)[0]} events={';'.join(evs)}")
        out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} | {o}\n")
    out.close()
    print(f"death: {RUNS} cases")


if __name__ == "__main__":
    main()
