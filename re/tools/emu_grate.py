"""Record grates and fences (3900-3915) and the ghosts (5001, 5011).

grate.txt: SetupGrateSprite, then frames of HandleGrateSprite, each
followed at random by HitGrateSprite with an explosion (timer set or 0)
or Harry. gratebounce.txt: GrateBounce(object, grate, half, bounce, hot)
for objects all around a grate. ghost.txt: SetupGhostSprite, then frames
of HandleGhostSprite with Harry (gPlayerH/V) and the view moving, and
HitGhostSprite with a player shot at random frames.

Each line: "<name=value ...> -> <dump> | <outputs>"
Usage: python3 -I emu_grate.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from sprites import Faces, tvectors, dump  # noqa: E402

PLAYER_H, PLAYER_V, VIEW, SCORE = 0x100659B4, 0x100659B0, 0x100659BC, 0x10065AF8
SOUNDS = {0x10065BF4: 602, 0x10065BB4: 1200, 0x10065BD4: 850, 0x10065BD0: 851, 0x10065BCC: 852}
KINDS = [3900, 3905, 3910, 3915]
CASES = 15000


def s16(x):
    x &= 0xFFFF
    return x - 0x10000 if x & 0x8000 else x


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m, work)
    for f in ("InitGrateSprite", "InitGhostSprite"):
        m.call(f)
    tv = tvectors(f"{work}/ppc")
    by_name = {v: k for k, v in tv.items()}
    ev = []
    m.stub_routine("STPlayRegSound", lambda mm: ev.append("p:%d" % mm.get(3)) or 0)
    m.stub_routine("STPlay3DSound", lambda mm: ev.append("3d:%d" % mm.get(3)) or 0)

    def explosion(mm):
        ev.append("x:%d:%d:%d:%d" % tuple(s16(mm.get(i)) for i in (3, 4, 7, 8)))
        rec = mm.alloc(0xA2)
        ev.append(rec)
        return rec
    m.stub_routine("EasyExplosion", explosion)
    for slot, snd in SOUNDS.items():
        m.w32(m.glob(slot), snd)
    base = m.load_level(open(f"{work}/res/Episode_1/Hlvl_6.bin", "rb").read())
    sp, other = m.alloc(0xA2), m.alloc(0xA2)
    mark = m.heap
    rnd = random.Random("grate")
    fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())

    def events():
        # an explosion's record: its timer as the routine left it
        out = []
        for e in ev:
            if isinstance(e, int):
                out[-1] += ":t%d" % s16(m.r16(e + 0x74))
            else:
                out.append(e)
        return ",".join(out) or "-"

    out = open(f"{work}/diff/grate.txt", "w")
    out.write("# grates: name=value inputs -> dump | score events\n")
    for _ in range(CASES):
        m.heap = mark
        st = {"kind": rnd.choice(KINDS), "v": rnd.randint(200, 3800), "h": rnd.randint(200, 3800),
              "hp": rnd.choice([-1, -1, 0, 1, 2]), "frozen": int(rnd.random() < 0.02), "frames": rnd.randint(1, 300),
              "seed": rnd.randint(1, 0x7FFFFFFE)}
        hits = []
        for _f in range(st["frames"]):
            r = rnd.random()
            hits.append("-" if r < 0.7 else rnd.choice(["e0", "e5", "p", "p", "pf"]))
        st["hits"] = ",".join(hits)
        m.write(sp, bytes(0xA2))
        m.w16(sp, st["kind"])
        m.w16(sp + 2, st["v"])
        m.w16(sp + 4, st["h"])
        m.call("SetupGrateSprite", sp)
        if st["hp"] != -1:
            m.w16(sp + 0x72, st["hp"])
        m.write(sp + 0x92, bytes([st["frozen"]]))
        m.w32(m.glob(SCORE), 0)
        m.rand_seed = st["seed"]
        evs = []
        for h in hits:
            ev.clear()
            m.call("HandleGrateSprite", sp)
            if h != "-":
                m.write(other, bytes(0xA2))
                m.w32(other + 0x1A, by_name["HandleEffectSprite" if h[0] == "e" else "HandlePlayerSprite"])
                if h[0] == "e":
                    m.w16(other + 0x74, int(h[1:]))
                m.write(other + 0x92, bytes([int(h == "pf")]))
                m.call("HitGrateSprite", sp, other)
            evs.append(events())
        out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} | score={m.r32(m.glob(SCORE))} events={';'.join(evs)}\n")
    out.close()
    print(f"grate: {CASES} cases")

    out = open(f"{work}/diff/gratebounce.txt", "w")
    out.write("# GrateBounce: name=value inputs -> grate dump | object outputs\n")
    hot = m.alloc(8)
    for _ in range(CASES * 2):
        kind = rnd.choice(KINDS)
        gv, gh = rnd.randint(300, 3700), rnd.randint(300, 3700)
        half = rnd.choice([12, 18, 32])
        hr = rnd.choice([(0, 0, 24, 24), (8, 8, 28, 28), (4, 4, 24, 24), (22, 22, 42, 42)])
        st = {"kind": kind, "gv": gv, "gh": gh, "half": half, "hot": ":".join(map(str, hr)),
              "v": gv + rnd.randint(-60, 60), "h": gh + rnd.randint(-60, 60),
              "speedv": rnd.randint(-900, 900), "speedh": rnd.randint(-900, 900), "bounce": rnd.choice([150, 255, 100])}
        st["fixh"], st["fixv"] = st["h"] * 256 + rnd.randint(0, 255), st["v"] * 256 + rnd.randint(0, 255)
        m.write(other, bytes(0xA2))
        m.w16(other, kind)
        m.w16(other + 2, gv)
        m.w16(other + 4, gh)
        m.call("SetupGrateSprite", other)
        m.write(sp, bytes(0xA2))
        for off, k in ((0x5A, "v"), (0x5C, "h"), (0x4E, "speedv"), (0x50, "speedh")):
            m.w16(sp + off, st[k])
        m.w32(sp + 0x5E, st["fixh"])
        m.w32(sp + 0x62, st["fixv"])
        m.write(hot, struct.pack(">hhhh", *hr))
        m.call("GrateBounce", sp, other, half, st["bounce"], hot)
        o = {"v": s16(m.r16(sp + 0x5A)), "h": s16(m.r16(sp + 0x5C)), "speedv": s16(m.r16(sp + 0x4E)),
             "speedh": s16(m.r16(sp + 0x50)), "fixh": struct.unpack(">i", m.read(sp + 0x5E, 4))[0],
             "fixv": struct.unpack(">i", m.read(sp + 0x62, 4))[0]}
        out.write(f"{fmt(st)} -> {dump(m, other, faces, tv)} | {fmt(o)}\n")
    out.close()
    print(f"gratebounce: {CASES * 2} cases")

    out = open(f"{work}/diff/ghost.txt", "w")
    out.write("# ghosts: name=value inputs -> dump | entry score events\n")
    view = m.glob(VIEW)
    for _ in range(CASES):
        m.heap = mark
        m.write(base + 0x10000, open(f"{work}/res/Episode_1/Hlvl_6.bin", "rb").read()[0x10000:0x11000])
        v, h = rnd.randint(400, 3600), rnd.randint(400, 3600)
        st = {"kind": rnd.choice([5001, 5011]), "v": v, "h": h, "index": rnd.choice([-1, rnd.randint(0, 255)]),
              "pv": v + rnd.randint(-300, 300), "ph": h + rnd.randint(-300, 300),
              "hp": rnd.choice([-1, -1, 0, 1, -1]), "mode": rnd.choice([-1, -1, 0, 1, 4]),
              "frozen": int(rnd.random() < 0.02), "frames": rnd.randint(1, 120), "seed": rnd.randint(1, 0x7FFFFFFE)}
        st["vv"], st["vh"] = st["pv"] - 200 + rnd.randint(-40, 40), st["ph"] - 200 + rnd.randint(-40, 40)
        steps = []
        for _f in range(st["frames"]):
            steps.append(f"{rnd.randint(-4, 4)}:{rnd.randint(-4, 4)}:" + rnd.choice(["-"] * 8 + ["s", "sf"]))
        st["steps"] = ",".join(steps)
        m.w16(m.glob(PLAYER_H), st["ph"])
        m.w16(m.glob(PLAYER_V), st["pv"])
        m.w16(view, st["vv"])
        m.w16(view + 2, st["vh"])
        m.w32(m.glob(SCORE), 0)
        m.write(sp, bytes(0xA2))
        m.w16(sp, st["kind"])
        m.w16(sp + 2, v)
        m.w16(sp + 4, h)
        m.call("SetupGhostSprite", sp)
        m.w16(sp + 0x68, st["index"])
        if st["hp"] != -1:
            m.w16(sp + 0x72, st["hp"])
        if st["mode"] != -1:
            m.w32(sp + 0x6E, st["mode"])
        m.write(sp + 0x92, bytes([st["frozen"]]))
        m.rand_seed = st["seed"]
        evs = []
        for step in steps:
            dh, dv, shot = step.split(":")
            m.w16(m.glob(PLAYER_H), m.r16(m.glob(PLAYER_H)) + int(dh))
            m.w16(m.glob(PLAYER_V), m.r16(m.glob(PLAYER_V)) + int(dv))
            m.w16(view, m.r16(view) + int(dv))
            m.w16(view + 2, m.r16(view + 2) + int(dh))
            ev.clear()
            m.call("HandleGhostSprite", sp)
            if shot != "-":
                m.write(other, bytes(0xA2))
                m.w32(other + 0x1A, by_name["HandlePlayerShotSprite"])
                m.write(other + 0x92, bytes([int(shot == "sf")]))
                m.call("HitGhostSprite", sp, other)
                ev.append("shot:%d" % m.read(other + 0x92, 1)[0])
            evs.append(events())
        e = m.read(base + 0x10000 + st["index"] * 16, 1)[0] if st["index"] >= 0 else -1
        out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} | entry={e} score={m.r32(m.glob(SCORE))} events={';'.join(evs)}\n")
    out.close()
    print(f"ghost: {CASES} cases")


if __name__ == "__main__":
    main()
