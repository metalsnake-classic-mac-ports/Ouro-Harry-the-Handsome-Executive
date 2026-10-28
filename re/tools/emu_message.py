"""Record the message sprite (memos and invisible message spots, 21000+):
SetupMessageSprite, frames of HandleMessageSprite, HitMessageSprite with
Harry. ShowBalloon is stubbed: it answers "shown" or not (an input per
call) and, as the real one does, freezes the sprite when shown (unless
mode 5); GenerateBuzzwords is stubbed.

Each line: "<name=value ...> -> <dump> | pending=<0/1> calls=<...>"
  calls   b:<text>:<mode>:<answer> per ShowBalloon call, text "buzz" for
          the buzzword generator, else the text number (level info #0)
Usage: python3 -I emu_message.py [re/work]
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from sprites import Faces, tvectors, dump  # noqa: E402

TV_PLAYER, TV_OTHER, PREFS, WORLD = 0x10066FC4, 0x1006702C, 0x100659A0, 0x10065B68
CASES = 12000


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m)
    m.call("InitMessageSprite")
    tv = tvectors(f"{work}/ppc")
    lvl = m.load_level(open(f"{work}/res/Episode_1/Hlvl_1.bin", "rb").read())
    # the world's texts: a handle to a block; texts at +0x3c9 + n * 256
    world = m.alloc(0x3C9 + 256 * 128)
    wh = m.alloc(4)
    m.w32(wh, world)
    m.w32(m.glob(WORLD), wh)
    calls, answers = [], []

    def show(mm):
        text, mode = mm.get(4), mm.get(5) & 0xFFFF
        inside = world <= text < world + 0x3C9 + 256 * 128
        name = str((text - world - 0x3C9) // 256) if inside else "buzz"
        ans = answers.pop(0) if answers else 0
        calls.append(f"b:{name}:{mode}:{ans}")
        if ans and mm.get(3) and mode != 5:
            mm.write(mm.get(3) + 0x92, b"\1")
        return ans
    m.stub_routine("ShowBalloon", show)

    m.stub_routine("GenerateBuzzwords", 0)       # fills a buffer on the caller's stack
    prefs = m.glob(PREFS)
    sp, other = m.alloc(0xA2), m.alloc(0xA2)
    rnd = random.Random("message")
    out = open(f"{work}/diff/message.txt", "w")
    out.write("# message: name=value inputs -> dump | pending calls\n")
    for case in range(CASES):
        st = {"kind": rnd.choice([21000, 21000, 21001, 21500]), "h": rnd.randint(0, 4000), "v": rnd.randint(0, 4000),
              "levelindex": rnd.choice([-1, rnd.randint(0, 255)]), "info0": rnd.randint(0, 100),
              "info1": rnd.choice([0, 0, 1, 2, 3, 5]), "frames": rnd.choice([0, 1, 2, rnd.randint(0, 40)]),
              "hitframe": -1, "frozen": int(rnd.random() < 0.03), "other": rnd.choice([1, 1, 1, 2, 3]),
              "kicking": int(rnd.random() < 0.2), "pref8": rnd.choice([0, 1, 2, 3, 3]),
              "answers": "".join(rnd.choice("0011") for _ in range(4))}
        st["hitframe"] = rnd.randint(0, st["frames"])
        m.write(sp, bytes(0xA2))
        m.w16(sp, st["kind"])
        m.w16(sp + 2, st["v"])
        m.w16(sp + 4, st["h"])
        m.call("SetupMessageSprite", sp)
        m.w16(sp + 0x68, st["levelindex"])
        if st["levelindex"] >= 0:
            m.w16(lvl + 0x10004 + st["levelindex"] * 16, st["info0"])
            m.w16(lvl + 0x10006 + st["levelindex"] * 16, st["info1"])
        m.w16(prefs + 8, st["pref8"])
        m.write(other, bytes(0xA2))
        m.w32(other + 0x1A, TV_OTHER if st["other"] == 2 else TV_PLAYER)
        m.write(other + 0x92, b"\1" if st["other"] == 3 else b"\0")
        m.w16(other + 8, 0 if st["kicking"] else 22)
        answers[:] = [int(c) for c in st["answers"]]
        calls.clear()
        for f in range(st["frames"] + 1):
            if f == st["hitframe"]:
                if f == 0:
                    m.write(sp + 0x92, bytes([st["frozen"]]))
                m.call("HitMessageSprite", sp, other)
            if f < st["frames"]:
                m.call("HandleMessageSprite", sp)
        pending = int(m.r32(sp + 0x6A) != 0)
        fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())
        out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} | pending={pending} calls={','.join(calls) or '-'}\n")
    out.close()
    print(f"message: {CASES} cases")


if __name__ == "__main__":
    main()
