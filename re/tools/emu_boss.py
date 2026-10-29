"""Record the boss (2201): SetupBossSprite at its place in level 15 (and
nearby), its globals (phase, sweep count, spin, angle, cooldowns) and hit
points set at random, then frames of HandleBossSprite (and, once it dies,
HandleBossSpriteDeath) with Harry and the view moving, each followed at
random by HitBossSprite with a player shot, an explosion or Harry kicking.
A ghost stands in the sprite list (the death takes the ghosts' hit
points). SATNewSprite is stubbed (a zeroed record with pos and fix as the
setups leave them); what the boss sets on it is logged.

Each line: "<name=value ...> -> <dump> | <globals> ghost=<its hit points> events=<per frame, ';'>"
  events  shot:<kind>:<h>:<v>:<speed v>:<speed h>:<dir>:<fix h>:<fix v>[:t<timer>:f<f7c> for 1211],
          ghost:<kind>:<h>:<v>:<dir>, big:<h>:<v>, x:<h>:<v>:<kind>:<dir>, 3d:<snd>[:<rate>],
          stop:<snd>, fade
Usage: python3 -I emu_boss.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from sprites import Faces, tvectors, dump  # noqa: E402

SAT, PLAYER_H, PLAYER_V, VIEW, SCORE, SHAKE, CONTACT = (0x10065B70, 0x100659B4, 0x100659B0, 0x100659BC, 0x10065AF8,
                                                       0x10065AD0, 0x10065E1C)
# (name, TOC slot or address, size): the boss's globals
GLOBALS = [("phase", 0x1006C350, 2, True), ("growing", 0x1006C352, 1, True), ("change", 0x10065F80, 2, False),
           ("anim", 0x10065F7C, 2, False), ("count", 0x10065F5C, 2, False), ("spin", 0x10065F58, 2, False),
           ("angle", 0x10065F54, 4, False), ("cooldown", 0x10065F3C, 2, False), ("blast", 0x10065F44, 2, False)]
SOUNDS = {0x10065B40: 4901, 0x10065C18: 300, 0x10065BC0: 1002, 0x10065BC8: 1000, 0x10065BC4: 1001,
          0x10065BFC: 600, 0x10065BF4: 602}
RUNS = 4000


def s16(x):
    x &= 0xFFFF
    return x - 0x10000 if x & 0x8000 else x


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m, work)
    m.stub_routine(0x1005F340, lambda mm: mm.alloc(0x40))
    for f in ("InitBossSprite", "InitTileHotRects", "InitPlayerKickRects"):
        m.call(f)
    tv = tvectors(f"{work}/ppc")
    by_name = {v: k for k, v in tv.items()}
    ev, news = [], []
    m.stub_routine("STPlayRegSound", lambda mm: ev.append("p:%d" % mm.get(3)) or 0)
    m.stub_routine("STPlay3DSound", lambda mm: ev.append("3d:%d" % mm.get(3)) or 0)
    m.stub_routine("STPlay3DSoundPitched", lambda mm: ev.append("3d:%d:%d" % (mm.get(3), mm.get(7))) or 0)
    m.stub_routine("EasyExplosion", lambda mm: ev.append("x:%d:%d:%d:%d" % tuple(s16(mm.get(i)) for i in (3, 4, 7, 8))) or 0)
    m.stub_routine("FadeMusic", lambda mm: ev.append("fade") or 0)
    m.stub_routine(0x10056C0C, lambda mm: ev.append("stop:%d" % mm.get(3)) or 0)
    m.stub_routine(0x10056DE8, 0)
    for f in ("GammaFadeOut", "GammaFadeIn", 0x1002549C, "AddFGRedraw"):
        m.stub_routine(f, 0)

    def new_sprite(mm):
        rec = mm.alloc(0xA2)
        kind, h, v = s16(mm.get(3)), s16(mm.get(4)), s16(mm.get(5))
        mm.w16(rec, kind)
        mm.w16(rec + 2, v)
        mm.w16(rec + 4, h)
        mm.w32(rec + 0x5E, (h << 8) & 0xFFFFFFFF)
        mm.w32(rec + 0x62, (v << 8) & 0xFFFFFFFF)
        news.append((rec, kind, h, v))
        ev.append(rec)
        return rec
    m.stub_routine("SATNewSprite", new_sprite)
    for slot, snd in SOUNDS.items():
        m.w32(m.glob(slot), snd)

    def gaddr(slot, direct):
        return slot if direct else m.glob(slot)

    def frame_events():
        out = []
        made = {n[0]: n for n in news}
        for e in ev:
            if not isinstance(e, int):
                out.append(e)
                continue
            _, kind, h, v = made[e]
            if kind == 0:
                if s16(m.r16(e + 0x4C)) == 500:
                    out.append(f"big:{h}:{v}")
                continue
            if kind == 5011:
                out.append(f"ghost:{kind}:{h}:{v}:{s16(m.r16(e + 0x66))}")
                continue
            sh = "shot:%d:%d:%d:%d:%d:%d:%d:%d" % (
                kind, h, v, s16(m.r16(e + 0x4E)), s16(m.r16(e + 0x50)), s16(m.r16(e + 0x66)),
                struct.unpack(">i", m.read(e + 0x5E, 4))[0], struct.unpack(">i", m.read(e + 0x62, 4))[0])
            if kind == 1211:
                sh += ":t%d:f%d" % (s16(m.r16(e + 0x74)), s16(m.r16(e + 0x7C)))
            out.append(sh)
        return ",".join(out) or "-"

    data = open(f"{work}/res/Episode_1/Hlvl_15.bin", "rb").read()
    m.load_level(data)
    places = []
    for i in range(256):
        e = data[0x10000 + i * 16:0x10000 + i * 16 + 16]
        if e[0] and 2200 <= struct.unpack(">h", e[2:4])[0] <= 2299:
            v, h = struct.unpack(">hh", e[12:16])
            places.append((i, v - 6, h - 6))
    sat = m.glob(SAT)
    view = m.glob(VIEW)
    sp, other, ghost = m.alloc(0xA2), m.alloc(0xA2), m.alloc(0xA2)
    mark = m.heap
    rnd = random.Random("boss")
    fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())
    out = open(f"{work}/diff/boss.txt", "w")
    out.write("# boss: name=value inputs -> dump | globals ghost events\n")
    for _ in range(RUNS):
        m.heap = mark
        index, v, h = rnd.choice(places)
        st = {"index": index, "v": v + rnd.choice([0, 0, rnd.randint(-80, 80)]), "h": h + rnd.choice([0, 0, rnd.randint(-80, 80)]),
              "hp": rnd.choice([-1, -1, 150, 101, 100, 99, 51, 50, 49, 9, 1, 0]),
              "count": rnd.choice([-1, -1, 0, 1, 59, 101, 119, 121, -46, -16]),
              "spin": rnd.choice([-1, -1, 0, 199, 201, -201, 599, -599, 300]),
              "cooldown": rnd.choice([-1, -1, 0, 3]), "change": rnd.choice([-1, -1, -1, -100, -98, -3]),
              "phase": rnd.choice([-1, -1, 1, 2, 3]), "growing": rnd.choice([0, 1]), "anim": rnd.randint(0, 7),
              "blast": rnd.choice([0, 0, 5]), "scale": rnd.choice([256, 256, 300, 255, 0x1c1]),
              "frames": rnd.randint(1, 400), "seed": rnd.randint(1, 0x7FFFFFFE)}
        st["pv"], st["ph"] = st["v"] + rnd.randint(-200, 200), st["h"] + rnd.randint(-200, 200)
        st["vv"], st["vh"] = st["pv"] - 200 + rnd.randint(-100, 100), st["ph"] - 200 + rnd.randint(-100, 100)
        steps = []
        for _f in range(st["frames"]):
            hit = "-"
            if rnd.random() < 0.15:
                hit = rnd.choice(["shot", "shot", "shot", "fx", "fx0", "kick"])
            steps.append(f"{rnd.randint(-4, 4)}:{rnd.randint(-4, 4)}:{hit}:{rnd.randint(0, 35)}")
        st["steps"] = ",".join(steps)
        m.w16(m.glob(PLAYER_H), st["ph"])
        m.w16(m.glob(PLAYER_V), st["pv"])
        m.w16(view, st["vv"])
        m.w16(view + 2, st["vh"])
        m.w32(m.glob(SCORE), 0)
        m.w16(m.glob(SHAKE), 0)
        m.w16(m.glob(CONTACT), 0)
        m.write(gaddr(0x1006C352, True), bytes([st["growing"]]))
        m.w16(m.glob(0x10065F7C), st["anim"])
        m.w16(m.glob(0x10065F44), st["blast"])
        m.w16(m.glob(0x10065F3C), 0)                # the setup leaves the shot cooldown
        m.write(sp, bytes(0xA2))
        m.w16(sp, 2201)
        m.w16(sp + 2, st["v"])
        m.w16(sp + 4, st["h"])
        m.rand_seed = st["seed"]
        m.call("SetupBossSprite", sp)
        m.w16(sp + 0x68, index)
        for k, slot in (("count", 0x10065F5C), ("spin", 0x10065F58), ("cooldown", 0x10065F3C), ("change", 0x10065F80)):
            if st[k] != -1:
                m.w16(m.glob(slot), st[k])
        if st["phase"] != -1:
            m.w16(0x1006C350, st["phase"])
        if st["hp"] != -1:
            m.w16(sp + 0x72, st["hp"])
        m.w16(sp + 0x52, st["scale"])
        m.w16(sp + 0x54, st["scale"])
        # a ghost in the sprite list
        m.write(ghost, bytes(0xA2))
        m.w32(ghost + 0x1A, by_name["HandleGhostSprite"])
        m.w16(ghost + 0x72, 3)
        m.w32(sat + 0x78, ghost)
        evs = []
        for step in steps:
            dh, dv, hit, odir = step.split(":")
            for a, d in ((m.glob(PLAYER_H), int(dh)), (m.glob(PLAYER_V), int(dv)), (view, int(dv)), (view + 2, int(dh))):
                m.w16(a, m.r16(a) + d)
            ev.clear()
            news.clear()
            task = tv.get(m.r32(sp + 0x1A))
            m.call(task, sp)
            if hit != "-" and m.r32(sp + 0x1E):
                m.write(other, bytes(0xA2))
                ov, oh = s16(m.r16(sp + 0x5A)) + 8, s16(m.r16(sp + 0x5C)) + 8
                if hit == "kick":
                    ov, oh = ov - 40, oh - 40
                t = {"shot": "HandlePlayerShotSprite", "fx": "HandleEffectSprite", "fx0": "HandleEffectSprite",
                     "kick": "HandlePlayerSprite"}[hit]
                m.w32(other + 0x1A, by_name[t])
                if hit == "fx":
                    m.write(other + 6, struct.pack(">hhhh", 4, 4, 60, 60))
                m.w16(other + 0x66, int(odir))
                m.w16(other + 0x5A, ov)
                m.w16(other + 0x5C, oh)
                m.call("HitBossSprite", sp, other)
                ev.append("o:%d" % m.read(other + 0x92, 1)[0])
            evs.append(frame_events())
        g = []
        for name, slot, size, direct in GLOBALS:
            a = gaddr(slot, direct)
            val = m.read(a, 1)[0] if size == 1 else s16(m.r16(a)) if size == 2 else struct.unpack(">i", m.read(a, 4))[0]
            g.append(f"{name}={val}")
        o = (" ".join(g) + f" score={m.r32(m.glob(SCORE))} shake={s16(m.r16(m.glob(SHAKE)))} "
             f"contact={s16(m.r16(m.glob(CONTACT)))} ghost={s16(m.r16(ghost + 0x72))} events={';'.join(evs)}")
        out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)} | {o}\n")
    out.close()
    print(f"boss: {RUNS} cases")


if __name__ == "__main__":
    main()
