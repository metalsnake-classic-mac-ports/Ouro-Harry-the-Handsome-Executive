"""Record buttons (9501-9541) and sliding doors (951, 961).

button.txt: SetupButtonSprite, then frames of HandleButtonSprite, each
followed by HitButtonSprite with a random partner (Harry walking or
kicking, plant, barrel, chair, mail guy, player / enemy shot, a bonus) or none.
sliding.txt: SetupSlidingDoorSprite, then frames of
HandleSlidingDoorSprite with the linked entry's info #1 changed at random
between frames.

Each line: "<name=value ...> -> <dump> | <outputs>"
  hits    per frame "-" or "<partner>:<v>:<h>:<dir>:<kicking>", ","-joined
  events  per frame "p:<snd>" / "3d:<snd>", ";" between frames
Usage: python3 -I emu_button.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from sprites import Faces, tvectors, dump  # noqa: E402

SOUNDS = {0x10065B8C: 4709, 0x10065B90: 4708, 0x10065B94: 4707, 0x10065B98: 4706, 0x10065B9C: 4705, 0x10065BA0: 4704}
BUTTONS = [9501, 9511, 9521, 9531, 9541]
PARTNERS = ["harry", "kick", "plant", "barrel", "chair", "mailguy", "robot", "playershot", "enemyshot", "bonus"]
CASES = 15000


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m)
    m.stub_routine(0x1005F340, lambda mm: mm.alloc(0x40))
    for f in ("InitButtonSprite", "InitSlidingDoorSprite", "InitPlayerKickRects"):
        m.call(f)
    tv = tvectors(f"{work}/ppc")
    by_name = {v: k for k, v in tv.items()}
    ev = []
    m.stub_routine("STPlayRegSound", lambda mm: ev.append("p:%d" % mm.get(3)) or 0)
    m.stub_routine("STPlay3DSound", lambda mm: ev.append("3d:%d" % mm.get(3)) or 0)
    for slot, snd in SOUNDS.items():
        m.w32(m.glob(slot), snd)
    data = open(f"{work}/res/Episode_1/Hlvl_2.bin", "rb").read()
    base = m.load_level(data)
    sp, other = m.alloc(0xA2), m.alloc(0xA2)
    rnd = random.Random("button")
    fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())

    def entry(i):
        return base + 0x10000 + i * 16

    out = open(f"{work}/diff/button.txt", "w")
    out.write("# buttons: name=value inputs -> dump | info1 events\n")
    for _ in range(CASES):
        m.write(base + 0x10000, data[0x10000:0x11000])
        kind = rnd.choice(BUTTONS)
        v, h = rnd.randint(200, 3800), rnd.randint(200, 3800)
        index = rnd.choice([rnd.randint(0, 255), rnd.randint(0, 20)])
        st = {"kind": kind, "v": v, "h": h, "index": index, "info0": rnd.choice([0, 0, 1, 2]),
              "info1": rnd.choice([0, 0, 1, 5]), "frozen": int(rnd.random() < 0.03), "frames": rnd.randint(1, 20)}
        hits = []
        for _f in range(st["frames"]):
            if rnd.random() < 0.4:
                hits.append("-")
                continue
            p = rnd.choice(PARTNERS)
            hits.append(":".join(map(str, (p, v + rnd.randint(-60, 30), h + rnd.randint(-60, 30), rnd.randint(0, 35),
                                           int(rnd.random() < 0.05)))))
        st["hits"] = ",".join(hits)
        m.w16(entry(index) + 4, st["info0"])
        m.w16(entry(index) + 6, st["info1"])
        m.write(sp, bytes(0xA2))
        m.w16(sp, kind)
        m.w16(sp + 2, v)
        m.w16(sp + 4, h)
        m.call("SetupButtonSprite", sp)
        m.w16(sp + 0x68, index)
        m.write(sp + 0x92, bytes([st["frozen"]]))
        evs = []
        for hit in hits:
            ev.clear()
            m.call("HandleButtonSprite", sp)
            if hit != "-":
                p, ov, oh, odir, ofrozen = hit.split(":")
                m.write(other, bytes(0xA2))
                task = {"harry": "HandlePlayerSprite", "kick": "HandlePlayerSprite", "plant": "HandlePlantSprite",
                        "barrel": "HandleBarrelSprite", "chair": "HandleChairSprite", "mailguy": "HandleMailGuySprite", "robot": "HandleRobotSprite", "playershot": "HandlePlayerShotSprite",
                        "enemyshot": "HandleEnemyShotSprite", "bonus": "HandleBonusSprite"}[p]
                m.w32(other + 0x1A, by_name[task])
                m.w16(other + 0x5A, int(ov))
                m.w16(other + 0x5C, int(oh))
                m.w16(other + 0x66, int(odir))
                m.write(other + 0x92, bytes([int(ofrozen)]))
                m.write(other + 6, struct.pack(">hhhh", *((0, 0, 64, 64) if p == "kick" else (22, 22, 42, 42))))
                m.call("HitButtonSprite", sp, other)
            evs.append(",".join(ev) or "-")
        o = {"info1": m.r16(entry(index) + 6), "events": ";".join(evs)}
        out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} | {fmt(o)}\n")
    out.close()
    print(f"button: {CASES} cases")

    out = open(f"{work}/diff/sliding.txt", "w")
    out.write("# sliding doors: name=value inputs -> dump | events\n")
    for _ in range(CASES):
        m.write(base + 0x10000, data[0x10000:0x11000])
        kind = rnd.choice([951, 961, 961, 951, 950])
        v, h = rnd.randint(200, 3800), rnd.randint(200, 3800)
        index = rnd.choice([-1, rnd.randint(0, 255), rnd.randint(0, 20), 300])
        link = rnd.choice([rnd.randint(0, 255), -rnd.randint(1, 255), 0, rnd.randint(0, 20)])
        st = {"kind": kind, "v": v, "h": h, "index": index, "link": link, "frozen": int(rnd.random() < 0.03),
              "dir": rnd.choice([0, 0, rnd.randint(0, 14)]), "frames": rnd.randint(1, 25)}
        states = []
        for _f in range(st["frames"]):
            states.append(rnd.choice([0, 1, 1, 0, 3]) if rnd.random() < 0.3 or not states else states[-1])
        st["states"] = ",".join(map(str, states))
        m.write(sp, bytes(0xA2))
        m.w16(sp, kind)
        m.w16(sp + 2, v)
        m.w16(sp + 4, h)
        m.call("SetupSlidingDoorSprite", sp)
        m.w16(sp + 0x68, index)
        m.w16(sp + 0x66, st["dir"])
        m.write(sp + 0x92, bytes([st["frozen"]]))
        if 0 <= index < 256:
            m.w16(entry(index) + 4, link)
        target = abs(link)
        evs = []
        for s in states:
            if 0 <= index < 256:
                m.w16(entry(target) + 6, s)
            ev.clear()
            m.call("HandleSlidingDoorSprite", sp)
            evs.append(",".join(ev) or "-")
        out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} | events={';'.join(evs)}\n")
    out.close()
    print(f"sliding: {CASES} cases")


if __name__ == "__main__":
    main()
