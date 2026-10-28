"""Record the colliders (fans, phone, copier, magnet, wall guns, vending
machine, gate, the hot-rect-only ids): SetupColliderSprite, frames of
HandleColliderSprite (with HandleRepulsions over a SAT list of the
collider and up to four other sprites) and HitColliderSprite at one frame.

Stubs: STPlayRegSound / STPlay3DSound (logged), the sound-playing test
(answers "busy" per case), SATNewSprite (logged with the speed the
caller then sets), CopierSave and ShowBalloon (answers per case, logged),
GeneratePhonewords, DebugHook.

Each line: "<name=value ...> -> <dump> | <name=value ...>"
  t<i>     another sprite in the list: task:atV:atH:top:left:bottom:right:speedV:speedH
  other    the sprite of the hit: 1 Harry, 2 Harry frozen, 3 a bonus
  events   p:<snd>, 3d:<snd>, n:<kind>:<h>:<v>:<speed v>:<speed h>,
           save:<answer>, b:phone:<mode>:<answer>
Usage: python3 -I emu_collider.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from sprites import Faces, tvectors, dump, load_words, WORD_LAST  # noqa: E402

SAT, PLAYER_SPRITE, PLAYER_H, PLAYER_V = 0x10065B70, 0x10065980, 0x100659B4, 0x100659B0
STAPLE, BADASS, SHRAPNEL, CAFFEINE, COMFORT, MAGNET = 0x10065AEC, 0x10065AE8, 0x10065AE4, 0x10065AD4, 0x1006856A, 0x1006C4AC
SOUNDS = {0x10065C14: 301, 0x10065BF4: 602, 0x10065C4C: 250, 0x10065BE0: 700, 0x10065B88: 1301, 0x10065B84: 1302,
          0x10065B80: 1303}
TV = {"player": 0x10066FC4, "playershot": 0x100672FC, "enemyshot": 0x100671BC, "plant": 0x100672DC,
      "bonus": 0x1006702C, "door": 0x1006717C}
KINDS = [2301, 2302, 2303, 2304] * 3 + [3100] * 3 + [3101, 9199, 9200, 9200, 9300, 9300] + \
    [9401, 9411, 9421, 9431] * 2 + [9600] * 4 + [9800, 9800, 1019, 1029, 1039, 1049, 2001, 2000, 3000, 9402]
CASES = 20000


def s16(v):
    return ((v & 0xFFFF) ^ 0x8000) - 0x8000


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m)
    m.call("InitColliderSprite")
    tv = tvectors(f"{work}/ppc")
    data = open(f"{work}/res/Episode_1/Hlvl_1.bin", "rb").read()
    lvl = m.load_level(data)
    ev, answers = [], {}
    m.stub_routine("STPlayRegSound", lambda mm: ev.append("p:%d" % mm.get(3)) or 0)
    m.stub_routine("STPlay3DSound", lambda mm: ev.append("3d:%d" % mm.get(3)) or 0)
    m.stub_routine(0x10056DE8, lambda mm: answers["busy"])
    m.stub_routine("DebugHook", 0)
    load_words(m, work)

    def new_sprite(mm):
        rec = mm.alloc(0xA2)
        ev.append(("n", rec, s16(mm.get(3)), s16(mm.get(4)), s16(mm.get(5))))
        return rec
    m.stub_routine("SATNewSprite", new_sprite)

    def save(mm):
        ev.append("save:%d" % answers["save"])
        return answers["save"]
    m.stub_routine("CopierSave", save)

    def show(mm):
        t = mm.get(4)
        ev.append("b:phone:%s:%d:%d" % (mm.read(t + 1, mm.read(t, 1)[0]).hex(), mm.get(5) & 0xFFFF, answers["show"]))
        return answers["show"]
    m.stub_routine("ShowBalloon", show)
    for slot, snd in SOUNDS.items():
        m.w32(m.glob(slot), snd)
    harry = m.alloc(0xA2)
    m.w32(m.glob(PLAYER_SPRITE), harry)
    sat = m.glob(SAT)
    sp, other = m.alloc(0xA2), m.alloc(0xA2)
    targets = [m.alloc(0xA2) for _ in range(4)]
    mark = m.heap
    rnd = random.Random("collider")
    out = open(f"{work}/diff/collider.txt", "w")
    out.write("# collider: name=value inputs -> dump | name=value outputs\n")
    for case in range(CASES):
        m.heap = mark
        kind = rnd.choice(KINDS)
        v, h = rnd.randint(100, 3800), rnd.randint(100, 3800)
        fan = 2301 <= kind <= 2304
        index = rnd.choice([-1, rnd.randint(0, 255)]) if kind not in (9600,) else rnd.randint(0, 255)
        if kind == 9402:
            index = -1
        if fan and rnd.random() < 0.7:
            index = rnd.randint(0, 255)
        st = {"kind": kind, "h": h, "v": v, "seed": rnd.randint(1, 0x7FFFFFFE), "levelindex": index,
              "info0": rnd.choice([0, 0, rnd.randint(-255, 255), rnd.randint(-3, 5)]),
              "info1": rnd.choice([0, 0, 1, rnd.randint(-3, 5), rnd.randint(-300, 600)]),
              "linkinfo1": rnd.choice([0, 0, 1, 7]),
              "settimer": rnd.choice([-9999, -9999, rnd.randint(-30, 5), rnd.randint(-300, 300), 1, 2, 3]),
              "setdir": rnd.choice([-9999, -9999, 0, rnd.randint(0, 5)]),
              "setf6e": rnd.choice([-9999, -9999, -9999, rnd.randint(-50, 300)]) if fan else -9999,
              "ph": h + rnd.choice([rnd.randint(-120, 120), rnd.randint(-600, 600)]),
              "pv": v + rnd.choice([rnd.randint(-120, 120), rnd.randint(-600, 600)]),
              "staple": rnd.choice([0, 0, 5]), "badass": rnd.choice([0, 0, 3]), "shrapnel": rnd.choice([0, 0, 9]),
              "caffeine": rnd.choice([0, 0, 40]), "comfort": rnd.randint(0, 144), "magnet": rnd.choice([0, 0, 1, 2, -1]),
              "busy": rnd.choice([0, 0, 1]), "frames": rnd.choice([0, 1, 2, rnd.randint(0, 40), rnd.randint(0, 400)]),
              "frozenframe": rnd.choice([-1, -1, -1, rnd.randint(0, 40)]), "hitframe": -1,
              "other": rnd.choice([1, 1, 1, 2, 3]), "save": rnd.choice([0, 1]), "show": rnd.choice([0, 1]),
              "ntargets": rnd.choice([0, 1, 4]) if fan else 0}
        st["hitframe"] = rnd.choice([-1, rnd.randint(0, st["frames"])])
        if kind == 9300 and rnd.random() < 0.6:
            # Harry at the edges of the magnet's pull
            st["ph"] = h - 20 + rnd.choice([-1, 1]) * rnd.randint(90, 100)
            st["pv"] = v - 20 + rnd.randint(-3, 3)
        if fan and rnd.random() < 0.7:
            st["ph"], st["pv"] = h + rnd.randint(-300, 300), v + rnd.randint(-300, 300)
            st["info0"] = rnd.choice([0, 0, rnd.randint(-50, 200)])
        if kind == 3100 and rnd.random() < 0.5:
            # Harry at the phone while it rings
            st["settimer"] = rnd.randint(1, 270)
            st["frames"] = rnd.randint(0, 40)
            st["hitframe"] = rnd.randint(0, st["frames"])
        for i in range(st["ntargets"]):
            task = rnd.choice(list(TV))
            hot = rnd.choice([(22, 22, 42, 42), (0, 0, 64, 64), (-6, -6, 22, 22), (rnd.randint(-10, 10), rnd.randint(-10, 10), rnd.randint(10, 40), rnd.randint(10, 40))])
            tgv, tgh = v + rnd.randint(-200, 200), h + rnd.randint(-200, 200)
            if rnd.random() < 0.7:
                # in front of the fan: along its wind, a little to the side
                along, across = rnd.randint(-20, 160), rnd.randint(-60, 60)
                dv, dh = {2301: (-along, across), 2302: (along, across), 2303: (across, -along), 2304: (across, along)}[kind]
                tgv, tgh = v + dv - 16, h + dh - 16
            st[f"t{i}"] = ":".join(str(x) for x in (task, tgv, tgh, *hot,
                                                     rnd.randint(-2500, 2500), rnd.randint(-2500, 2500)))
        m.write(lvl + 0x10000, data[0x10000:0x11000])
        if index >= 0:
            e = lvl + 0x10000 + index * 16
            m.write(e, b"\1")
            m.w16(e + 2, kind)
            m.w16(e + 4, st["info0"])
            m.w16(e + 6, st["info1"])
            link = abs(st["info0"])
            if link != index and link < 256:
                m.w16(lvl + 0x10006 + link * 16, st["linkinfo1"])
        for slot, k in ((STAPLE, "staple"), (BADASS, "badass"), (SHRAPNEL, "shrapnel"), (CAFFEINE, "caffeine"),
                        (PLAYER_H, "ph"), (PLAYER_V, "pv")):
            m.w16(m.glob(slot), st[k])
        m.w16(COMFORT, st["comfort"])
        m.w16(MAGNET, st["magnet"])
        m.write(harry, bytes(0xA2))
        answers.update(busy=st["busy"], save=st["save"], show=st["show"])
        m.rand_seed = st["seed"]
        for a in WORD_LAST:
            m.w16(a, 0)
        m.write(sp, bytes(0xA2))
        m.w16(sp, kind)
        m.w16(sp + 2, v)
        m.w16(sp + 4, h)
        m.call("SetupColliderSprite", sp)
        m.w16(sp + 0x68, index)
        if st["settimer"] != -9999:
            m.w16(sp + 0x74, st["settimer"])
        if st["setdir"] != -9999:
            m.w16(sp + 0x66, st["setdir"])
        if st["setf6e"] != -9999:
            m.w32(sp + 0x6E, st["setf6e"] & 0xFFFFFFFF)
        # the SAT list: the collider, then the others
        chain = [sp]
        for i in range(st["ntargets"]):
            t = targets[i]
            task, *nums = st[f"t{i}"].split(":")
            av, ah, top, left, bottom, right, sv, sh = map(int, nums)
            m.write(t, bytes(0xA2))
            m.w32(t + 0x1A, TV[task])
            m.write(t + 6, struct.pack(">hhhh", top, left, bottom, right))
            m.w16(t + 0x5A, av)
            m.w16(t + 0x5C, ah)
            m.w16(t + 0x4E, sv)
            m.w16(t + 0x50, sh)
            chain.append(t)
        m.w32(sat + 0x78, chain[0])
        for a, b in zip(chain, chain[1:] + [0]):
            m.w32(a + 0x2E, b)
        m.write(other, bytes(0xA2))
        m.w32(other + 0x1A, TV["bonus"] if st["other"] == 3 else TV["player"])
        m.write(other + 0x92, b"\1" if st["other"] == 2 else b"\0")
        ev.clear()
        for f in range(st["frames"] + 1):
            if f == st["frozenframe"]:
                m.write(sp + 0x92, b"\1")
            if f == st["hitframe"]:
                m.call("HitColliderSprite", sp, other)
            if f < st["frames"]:
                m.call("HandleColliderSprite", sp)
                for j, e in enumerate(ev):
                    if isinstance(e, tuple):
                        _, rec, k, nh, nv = e
                        ev[j] = f"n:{k}:{nh}:{nv}:{m.r16(rec + 0x4E)}:{m.r16(rec + 0x50)}"
        outs = {}
        if index >= 0:
            e = lvl + 0x10000 + index * 16
            outs.update(present=m.read(e, 1)[0], id=m.r16(e + 2), e0=m.r16(e + 4), e1=m.r16(e + 6))
        outs.update(comfort=m.r16(COMFORT), magnet=m.r16(MAGNET), harryspeedv=m.r16(harry + 0x4E))
        for i in range(st["ntargets"]):
            outs[f"t{i}"] = f"{m.r16(targets[i] + 0x4E)}:{m.r16(targets[i] + 0x50)}"
        outs["events"] = ",".join(ev) or "-"
        fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())
        out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} | {fmt(outs)}\n")
    out.close()
    print(f"collider: {CASES} cases")


if __name__ == "__main__":
    main()
