"""Record HitPlayerShotSprite - a player shot (staple 0, B.A.D.A.S.S. 1,
soda can 4) hitting the kinds it reacts to: colliders, sliding doors,
dart enemies, co-workers, scientists, robots (small and big), blobs,
grates and fences, and others it passes (plants, Harry, effects).
Every partner is made by its real setup.

Each line: "<name=value ...> -> <shot dump> || <other dump> | favor events"
  events  p:/3d:<snd>, 3d:<snd>:<rate> (pitched), x:<h>:<v>:<kind>:<dir> (EasyExplosion)
Usage: python3 -I emu_playershot.py [re/work]
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from sprites import Faces, tvectors, dump  # noqa: E402

FAVOR = 0x1006856E
SOUNDS = {0x10065BB0: 4700, 0x10065BFC: 600, 0x10065BE8: 604, 0x10065BEC: 603, 0x10065BA8: 4702, 0x10065C08: 500,
          0x10065C38: 255, 0x10065C40: 253, 0x10065C3C: 254, 0x10065C34: 256}
PARTNERS = {"collider": ("SetupColliderSprite", [9200, 9600, 3100]), "sliding": ("SetupSlidingDoorSprite", [951, 961]),
            "dart": ("SetupDartEnemySprite", [1011, 1021]), "coworker": ("SetupCoWorkerSprite", [1301, 1321]),
            "scientist": ("SetupScientistSprite", [1111, 1231]), "robot": ("SetupRobotSprite", [1801, 1811, 2001, 2021]),
            "blob": ("SetupBlobSprite", [1701, 1711, 1721]), "grate": ("SetupGrateSprite", [3900, 3905, 3910, 3915]),
            "plant": ("SetupPlantSprite", [601]), "effect": ("SetupEffectSprite", [0])}
CASES = 30000


def s16(x):
    x &= 0xFFFF
    return x - 0x10000 if x & 0x8000 else x


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m, work)
    m.stub_routine(0x1005F340, lambda mm: mm.alloc(0x40))
    for f in ("InitPlayerShotSprite", "InitColliderSprite", "InitSlidingDoorSprite", "InitDartEnemySprite",
              "InitCoWorkerSprite", "InitScientistSprite", "InitRobotSprite", "InitBlobSprite", "InitGrateSprite",
              "InitPlantSprite", "InitEffectSprite"):
        m.call(f)
    tv = tvectors(f"{work}/ppc")
    ev = []
    m.stub_routine("STPlayRegSound", lambda mm: ev.append("p:%d" % mm.get(3)) or 0)
    m.stub_routine("STPlay3DSound", lambda mm: ev.append("3d:%d" % mm.get(3)) or 0)
    m.stub_routine("STPlay3DSoundPitched", lambda mm: ev.append("3d:%d:%d" % (mm.get(3), mm.get(7))) or 0)
    m.stub_routine("EasyExplosion", lambda mm: ev.append("x:%d:%d:%d:%d" % tuple(s16(mm.get(i)) for i in (3, 4, 7, 8))) or 0)
    for slot, snd in SOUNDS.items():
        m.w32(m.glob(slot), snd)
    m.w32(m.glob(0x100659B4), 0)                    # gPlayerH / V: the robots face Harry at 0, 0
    sp, other = m.alloc(0xA2), m.alloc(0xA2)
    rnd = random.Random("playershot")
    fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())
    out = open(f"{work}/diff/playershot.txt", "w")
    out.write("# HitPlayerShotSprite: name=value inputs -> shot || other | favor events\n")
    for _ in range(CASES):
        p = rnd.choice(list(PARTNERS))
        setup, kinds = PARTNERS[p]
        v, h = rnd.randint(200, 3800), rnd.randint(200, 3800)
        st = {"shot": rnd.choice([0, 1, 4]), "v": v, "h": h, "other": p, "okind": rnd.choice(kinds),
              "ov": v + rnd.randint(-20, 20), "oh": h + rnd.randint(-20, 20), "frozen": int(rnd.random() < 0.03),
              "ofrozen": int(rnd.random() < 0.03), "ostate": rnd.choice([-1, -1, 0, 1]), "seed": rnd.randint(1, 0x7FFFFFFE),
              "seed2": rnd.randint(1, 0x7FFFFFFE)}
        m.rand_seed = st["seed"]
        m.write(sp, bytes(0xA2))
        m.w16(sp, st["shot"])
        m.w16(sp + 2, v)
        m.w16(sp + 4, h)
        m.call("SetupPlayerShotSprite", sp)
        m.write(other, bytes(0xA2))
        m.w16(other, st["okind"])
        m.w16(other + 2, st["ov"])
        m.w16(other + 4, st["oh"])
        m.call(setup, other)
        if st["ostate"] != -1:
            m.w16(other + 0x72, st["ostate"])
        m.write(sp + 0x92, bytes([st["frozen"]]))
        m.write(other + 0x92, bytes([st["ofrozen"]]))
        m.w16(FAVOR, 100)
        m.rand_seed = st["seed2"]
        ev.clear()
        m.call("HitPlayerShotSprite", sp, other)
        out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} || {dump(m, other, faces, tv)} | "
                  f"favor={s16(m.r16(FAVOR))} events={','.join(ev) or '-'}\n")
    out.close()
    print(f"playershot: {CASES} cases")


if __name__ == "__main__":
    main()
