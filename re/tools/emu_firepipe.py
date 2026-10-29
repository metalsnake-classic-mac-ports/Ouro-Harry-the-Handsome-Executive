"""Record the fire pipes (4201-4204) and their flames.

firepipe.txt: SetupFirePipeSprite, then frames of HandleFirePipeSprite
(SATNewSprite stubbed: the flame is logged with the kind and f7c the pipe
gives it). flame.txt: SetupEffectSprite with a pipe's kind, the kind and
f7c set as the pipe does, then frames of HandleEffectSprite - Harry
touching it (f6e = 3) at one frame.

Each line: "<name=value ...> -> <dump> [| events=<per frame, ';'>]"
Usage: python3 -I emu_firepipe.py [re/work]
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from sprites import Faces, tvectors, dump  # noqa: E402

SOUNDS = {0x10065BE0: 700}
CASES = 15000


def s16(x):
    x &= 0xFFFF
    return x - 0x10000 if x & 0x8000 else x


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m)
    for f in ("InitFirePipeSprite", "InitEffectSprite"):
        m.call(f)
    tv = tvectors(f"{work}/ppc")
    ev, news = [], []
    m.stub_routine("STPlay3DSound", lambda mm: ev.append("3d:%d" % mm.get(3)) or 0)

    def new_sprite(mm):
        rec = mm.alloc(0xA2)
        news.append((rec, s16(mm.get(3)), s16(mm.get(4)), s16(mm.get(5))))
        ev.append(rec)
        return rec
    m.stub_routine("SATNewSprite", new_sprite)
    for slot, snd in SOUNDS.items():
        m.w32(m.glob(slot), snd)
    base = m.load_level(open(f"{work}/res/Episode_1/Hlvl_5.bin", "rb").read())
    sp = m.alloc(0xA2)
    mark = m.heap
    rnd = random.Random("firepipe")
    fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())
    out = open(f"{work}/diff/firepipe.txt", "w")
    out.write("# fire pipe: name=value inputs -> dump | events\n")
    for _ in range(CASES):
        m.heap = mark
        st = {"kind": rnd.choice([4201, 4202, 4203, 4204]), "v": rnd.randint(200, 3800), "h": rnd.randint(200, 3800),
              "index": rnd.choice([-1, rnd.randint(0, 255)]), "info1": rnd.choice([0, 0, 0, 7, 4300]),
              "f6e": rnd.choice([-1, -1, 80, 90, 91, 200]), "frozen": int(rnd.random() < 0.03),
              "frames": rnd.randint(1, 300), "seed": rnd.randint(1, 0x7FFFFFFE)}
        if st["index"] >= 0:
            m.w16(base + 0x10000 + st["index"] * 16 + 6, st["info1"])
        m.write(sp, bytes(0xA2))
        m.w16(sp, st["kind"])
        m.w16(sp + 2, st["v"])
        m.w16(sp + 4, st["h"])
        m.rand_seed = st["seed"]
        m.call("SetupFirePipeSprite", sp)
        m.w16(sp + 0x68, st["index"])
        if st["f6e"] != -1:
            m.w32(sp + 0x6E, st["f6e"])
        m.write(sp + 0x92, bytes([st["frozen"]]))
        evs = []
        for _f in range(st["frames"]):
            ev.clear()
            news.clear()
            m.call("HandleFirePipeSprite", sp)
            made = {n[0]: n for n in news}
            evs.append(",".join(
                "flame:%d:%d:%d:%d" % (made[e][1], made[e][2], made[e][3], s16(m.r16(e))) + ("" if s16(m.r16(e + 0x7C)) == made[e][1] else ":bad")
                if isinstance(e, int) else e for e in ev) or "-")
        out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} | events={';'.join(evs)}\n")
    out.close()
    print(f"firepipe: {CASES} cases")

    out = open(f"{work}/diff/flame.txt", "w")
    out.write("# flame: name=value inputs -> dump\n")
    for _ in range(CASES):
        st = {"pipe": rnd.choice([4201, 4202, 4203, 4204, 4200, 4205]), "kind": rnd.choice([0, 0, 7]),
              "v": rnd.randint(200, 3800), "h": rnd.randint(200, 3800), "frames": rnd.randint(1, 80),
              "touch": rnd.choice([-1, -1, rnd.randint(0, 60)])}
        m.write(sp, bytes(0xA2))
        m.w16(sp, st["pipe"])
        m.w16(sp + 2, st["v"])
        m.w16(sp + 4, st["h"])
        m.call("SetupEffectSprite", sp)
        m.w16(sp, st["kind"])
        m.w16(sp + 0x7C, st["pipe"])
        for f in range(st["frames"]):
            if f == st["touch"]:
                m.w32(sp + 0x6E, 3)
            m.call("HandleEffectSprite", sp)
        out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)}\n")
    out.close()
    print(f"flame: {CASES} cases")


if __name__ == "__main__":
    main()
