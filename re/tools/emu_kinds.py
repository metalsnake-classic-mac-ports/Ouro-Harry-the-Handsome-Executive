"""Record simple sprite kinds: Setup...Sprite on a fresh (zeroed) record,
then a number of frames of its task, then its hit task with another
sprite. One file per kind in re/work/diff/kinds/<kind>.txt:

  "<kind> <h> <v> <frames> <frozenAt> <other> <extra> -> <dump> | <globals>"
  frozenAt  the frame before which frozen is set (-1 never)
  other     the sprite the hit task gets: 0 none (not called), 1 the
            player, 2 another sprite, 3 a frozen player, 4 a player
            shot, 5 an enemy shot
  extra     kind specific (exit: levelIndex:destination:level:timer;
            door: levelIndex:dir:hitTaskCleared)
  globals   exit: level counter, previous; door: level entry id, the
            other sprite's frozen flag
  dump      every modelled field after the frames and the hit
            (sprites.FIELDS, face as HrSp id, tasks by name)
Usage: python3 -I emu_kinds.py [re/work]
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from sprites import Faces, tvectors, dump  # noqa: E402

LEVEL_SLOT, CUR_LEVEL, PREV_LEVEL = 0x10065B64, 0x100659CC, 0x10068568
TV_PLAYER = 0x10066FC4
KINDS = {
    "bonus": ("InitBonusSprite", "SetupBonusSprite", "HandleBonusSprite", "HitBonusSprite",
              [700, 701, 702, 703, 711, 712, 713, 704, 750, 799]),
    "item": ("InitItemSprite", "SetupItemSprite", "HandleItemSprite", "HitItemSprite",
             [11199, 11299, 11397, 11398, 11399, 11499, 11599, 11699, 11799, 11899, 11999, 11701, 12099]),
    "exit": ("InitExitSprite", "SetupExitSprite", "HandleExitSprite", "HitExitSprite",
             [9990, 9991, 9992, 9993, 9994, 9995, 9996, 9999]),
    "door": ("InitDoorSprite", "SetupDoorSprite", "HandleDoorSprite", "HitDoorSprite",
             [901, 902, 903, 911, 912, 913, 921, 922, 923, 931, 932, 933, 900, 904, 934]),
}
TV_OTHER = {1: TV_PLAYER, 2: 0x10067194, 3: TV_PLAYER, 4: 0x100672FC, 5: 0x100671BC}
CASES = 3000


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    os.makedirs(f"{work}/diff/kinds", exist_ok=True)
    tv = tvectors(f"{work}/ppc")
    rnd = random.Random("kinds")
    for name, (init, setup, handle, hit, ids) in KINDS.items():
        m = Machine(f"{work}/ppc")
        faces = Faces(m)
        m.call(init)
        lvl = m.load_level(open(f"{work}/res/Episode_1/Hlvl_1.bin", "rb").read())
        cur, prev = m.glob(CUR_LEVEL), PREV_LEVEL
        sp, other = m.alloc(0xA2), m.alloc(0xA2)
        mark = m.heap
        out = open(f"{work}/diff/kinds/{name}.txt", "w")
        out.write(f"# {setup}, frames x {handle}, {hit}: kind h v frames frozenAt other extra -> dump | cur prev\n")
        for _ in range(CASES):
            m.heap = mark
            kind = rnd.choice(ids)
            h, v = rnd.randint(-50, 4100), rnd.randint(-50, 4100)
            frames = rnd.choice([rnd.randint(0, 6), rnd.randint(0, 40)])
            frozen_at = rnd.choice([-1, -1, -1, rnd.randint(0, frames)])
            oth = rnd.choice([0, 1, 1, 1, 2, 3] + ([4, 4, 5, 5] if name == "door" else []))
            m.write(sp, bytes(0xA2))
            m.w16(sp, kind)
            m.w16(sp + 2, v)
            m.w16(sp + 4, h)
            m.call(setup, sp)
            extra = "-"
            if name == "exit":
                idx, dest, level = rnd.randint(0, 255), rnd.choice([0, 0, rnd.randint(1, 15), -1]), rnd.randint(1, 15)
                m.w16(sp + 0x68, idx)
                m.w16(lvl + 0x10004 + idx * 16, dest)
                m.w16(cur, level)
                m.w16(prev, 0x5555)
                if rnd.random() < 0.3:
                    m.w16(sp + 0x74, rnd.randint(1, 69))
                extra = f"{idx}:{dest}:{level}:{m.r16(sp + 0x74)}"
            if name == "door":
                idx = rnd.choice([-1, rnd.randint(0, 255)])
                d = rnd.choice([m.r16(sp + 0x66), m.r16(sp + 0x66), 1, rnd.randint(0, 17)])
                cleared = int(rnd.random() < 0.1)
                m.w16(sp + 0x68, idx)
                m.w16(sp + 0x66, d)
                if cleared:
                    m.w32(sp + 0x1E, 0)
                if idx >= 0:
                    m.w16(lvl + 0x10002 + idx * 16, 0x4444)
                extra = f"{idx}:{d}:{cleared}"
            for f in range(frames):
                if f == frozen_at:
                    m.write(sp + 0x92, b"\1")
                m.call(handle, sp)
            if frozen_at == frames:
                m.write(sp + 0x92, b"\1")
            if oth:
                m.write(other, bytes(0xA2))
                m.w32(other + 0x1A, TV_OTHER[oth])
                m.write(other + 0x92, b"\1" if oth == 3 else b"\0")
                m.call(hit, sp, other)
            if name == "door":
                rec = m.r16(lvl + 0x10002 + idx * 16) if idx >= 0 else 0
                glob = f"{rec} {m.read(other + 0x92, 1)[0] if oth else 0}"
            else:
                glob = f"{m.r16(cur)} {m.r16(prev)}"
            out.write(f"{kind} {h} {v} {frames} {frozen_at} {oth} {extra} -> {dump(m, sp, faces, tv)} | {glob}\n")
        out.close()
    print("kinds written")


if __name__ == "__main__":
    main()
