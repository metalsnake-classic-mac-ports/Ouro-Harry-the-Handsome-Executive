"""Record HandleKeys - turning, the move keys, inventory selection, the
weapons and items, the cheat key sequences and the debug keys - with
the inventory routines it calls (SwitchInvItem, RemoveInvItem,
GetInvItem; their drawing stubbed).

Each line: "<inputs> -> <outputs>"; the fields are named in the first
line (# ...). "keys" is the pressed key codes joined by ",", "inv" the
12 slots as id:count joined by ",". "events" lists what the routine asked
for, in order (joined by ",", "-" for none):
  p:<snd>          STPlayRegSound            stop       stop the charge sound
  c:<snd>          ... kept as the charge    v:<snd>    ... kept as the voice
  3d:<snd>:<pitch> STPlay3DSoundPitched      x:<h>:<v>:<flag> EasyExplosion
  b:<n>            ShowBalloon, message n (-1 = the B-E-N one)
  shot:<h>:<v>:<dir>:<speedV>:<speedH>       SATNewSprite (player shot)
  sine             BuildSineTable
Usage: python3 -I emu_keys.py [re/work]
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402

TIMERS = [("scoot", 0x10065E84), ("kick", 0x10065E80), ("inch", 0x10065E7C), ("invswitch", 0x10065E4C),
          ("fire", 0x10065E48), ("itemuse", 0x10065E3C), ("recoil", 0x10065E38), ("caffeine", 0x10065AD4),
          ("cheat1", 0x10065E34), ("cheat2", 0x10065E30), ("wavy", 0x10065E2C), ("comfortdelta", 0x10065E6C),
          ("staple", 0x10065AEC), ("badass", 0x10065AE8), ("shrapnel", 0x10065AE4), ("hold", 0x10065EA4),
          ("step1", 0x10065E00), ("step2", 0x10065DFC), ("wavystep", 0x10065DF8), ("ripplephase", 0x1006592C)]
DIRECT = [("comfort", 0x1006856A), ("shotkind", 0x10068576), ("curslot", 0x1006857A)]
SPRITE = [("posV", 0x02), ("posH", 0x04), ("atV", 0x5A), ("atH", 0x5C), ("dir", 0x66), ("air", 0x8E),
          ("scale0", 0x52), ("scale1", 0x54), ("blast", 0x96)]
INVENTORY, CHATTER, PREFS, DEBUG, WAVY_ON, LEVEL = 0x10065AC4, 0x10065AC8, 0x100659A0, 0x10065B74, 0x10065E70, 0x10065B64
CHARGE, VOICE, VOICE2 = 0x10068A9C, 0x10068AA0, 0x10068AA4
BALLOONS, BEN = 0x10068BA8, 0x10068AA8
# the sound slots HandleKeys uses -> their snd ids (LoadSounds)
SOUNDS = {0x10065BF4: 602, 0x10065C5C: 241, 0x10065B50: 4801, 0x10065C64: 239, 0x10065C7C: 233,
          0x10065C80: 232, 0x10065C44: 252, 0x10065C94: 224, 0x10065C68: 238, 0x10065C88: 230,
          0x10065C84: 231, 0x10065C6C: 237, 0x10065CC8: 210, 0x10065C18: 300}
GULP = 241
# prefs +0x10..+0x1c: left, right, scoot, inch, kick, select, use (the defaults)
BINDINGS = [0x7B, 0x7C, 0x7D, 0x7E, 0x31, 0x3B, 0x3A]
FIXED = [0x12, 0x13, 0x14, 0x15, 0x38, 0x18, 0x74, 0x7A, 0x78, 0x6F, 0x69, 0x76, 0x63,
         0x23, 0x00, 0x0F, 0x05, 0x04, 0x06, 0x0B, 0x0E, 0x2D, 0x0D, 0x09, 0x10]
ITEMS = [11299, 11397, 11499, 11599, 11699, 11799, 11899, 11999]
CASES = 30000


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    ev = []
    channel = [0]

    def play(kind):
        def f(mm):
            channel[0] += 1
            ev.append((kind, mm.get(3)))
            return 0x1000 + channel[0]
        return f
    m.stub_routine("STPlayRegSound", play("p"))
    m.stub_routine("STPlay3DSoundPitched", lambda mm: ev.append(("3d", mm.get(3), mm.get(7))) or 0)
    busy = {}
    m.stub_routine(0x10056DE8, lambda mm: busy[mm.get(3)])
    m.stub_routine(0x10056C0C, lambda mm: ev.append(("stop", mm.get(3))) or 0)
    m.stub_routine("EasyExplosion", lambda mm: ev.append(("x", mm.get(3), mm.get(4), mm.get(7))) or 0)
    m.stub_routine("ShowBalloon", lambda mm: ev.append(("b", mm.get(4))) or 0)
    m.stub_routine("BuildSineTable", lambda mm: ev.append(("sine",)) or 0)
    shots = []

    def new_sprite(mm):
        s = mm.alloc(0xA2)
        shots.append((s, mm.get(3), mm.get(4), mm.get(5)))
        ev.append(("shot", len(shots) - 1))
        return s
    m.stub_routine("SATNewSprite", new_sprite)
    for f in ("UpdateAmmo", "CopyStatRect", "PlotEncFace", "DrawNumItems", "FindInvItemFace", "UpdateInventory",
              0x1005F738, 0x1005F7EC):
        m.stub_routine(f, 0)
    for n in ("ForeColor", "PaintRect"):
        m.stub(n, m.tb_nop)
    keymap = bytearray(16)
    m.stub("GetKeys", lambda: m.write(m.get(3), keymap))
    for slot, snd in SOUNDS.items():
        m.w32(m.glob(slot), snd)
    g = {n: m.glob(slot) for n, slot in TIMERS}
    inv = m.r32(INVENTORY)
    assert m.glob(CHATTER) == inv - 4
    prefs, debug, wavy_on = m.glob(PREFS), m.glob(DEBUG), m.glob(WAVY_ON)
    data = open(f"{work}/res/Episode_1/Hlvl_1.bin", "rb").read()
    lvl = m.load_level(data)
    sp = m.alloc(0xA2)
    rnd = random.Random("keys")
    names_in = ([n for n, _ in TIMERS + DIRECT + SPRITE] +
                ["chatter", "debug", "wavyon", "ripple0", "ripple1", "ripple2", "gulpbusy", "voicebusy", "seed",
                 "bindings", "keys", "inv"])
    names_out = ([n for n, _ in TIMERS + DIRECT + SPRITE] + ["wavyon", "ripple0", "ripple1", "ripple2", "inv", "events"])
    out = open(f"{work}/diff/keys.txt", "w")
    out.write("# HandleKeys: " + " ".join(names_in) + " -> " + " ".join(names_out) + "\n")
    for case in range(CASES):
        st = {}
        for n, _ in TIMERS:
            st[n] = rnd.choice([0, 0, 0, 1, rnd.randint(1, 30)])
        st["comfortdelta"] = rnd.choice([0, rnd.randint(-20, 20)])
        for n in ("staple", "badass", "shrapnel"):
            st[n] = rnd.choice([0, 1, 2, rnd.randint(0, 100)])
        st["hold"] = rnd.choice([0, 0, 1, 7, 8, 15, 16, 20, 21, 23, 24, 33, 34, rnd.randint(0, 60)])
        st["step1"] = rnd.randint(0, 7)
        st["step2"] = rnd.randint(0, 4)
        st["wavystep"] = rnd.randint(0, 10)
        st["ripplephase"] = rnd.randint(0, 50)
        st["comfort"] = rnd.choice([rnd.randint(0, 144), 9, 10, 11])
        st["shotkind"] = rnd.randint(0, 7)
        # the inventory: compact, now and then with a hole
        n_items = rnd.choice([0, 1, 2, rnd.randint(0, 12), 12])
        slots = [(rnd.choice(ITEMS), rnd.choice([1, 1, 2, 3])) for _ in range(n_items)] + [(-1, 0)] * (12 - n_items)
        if rnd.random() < 0.1:
            k = rnd.randint(0, 11)
            slots[k] = (-1, 0)
        st["curslot"] = (-1 if n_items == 0 or rnd.random() < 0.05 else rnd.randint(0, max(0, n_items - 1)))
        spr = {"posV": rnd.randint(0, 4000), "posH": rnd.randint(0, 4000), "dir": rnd.randint(0, 35),
               "air": rnd.choice([0, 0, 0, 1, 20, 999]), "scale0": rnd.choice([0x100, 0x100, 0x200]),
               "blast": rnd.choice([0, 0, 5])}
        spr["atV"], spr["atH"] = spr["posV"] + rnd.randint(-3, 3), spr["posH"] + rnd.randint(-3, 3)
        spr["scale1"] = spr["scale0"]
        chatter = rnd.choice([0, rnd.randint(0, 300)])
        dbg = 1 if rnd.random() < 0.1 else 0
        won = rnd.choice([0, 1])
        ripple = [rnd.randint(0, 100), rnd.randint(0, 16), rnd.randint(0, 0x300)]
        gulp_busy, voice_busy = rnd.choice([0, 1]), rnd.choice([0, 0, 1])
        seed = rnd.randint(1, 0x7FFFFFFE)
        bindings = BINDINGS if rnd.random() < 0.8 else [rnd.randint(0, 127) for _ in range(7)]
        p = rnd.choice([0.02, 0.08, 0.2])
        pressed = sorted({c for c in bindings + FIXED if rnd.random() < p})
        # write everything
        for n, _ in TIMERS:
            m.w16(g[n], st[n])
        for n, a in DIRECT:
            m.w16(a, st[n])
        m.write(sp, bytes(0xA2))
        for n, off in SPRITE:
            m.w16(sp + off, spr[n])
        for i, (oid, cnt) in enumerate(slots):
            m.w16(inv + 4 * i, oid)
            m.w16(inv + 4 * i + 2, cnt)
        m.w16(inv - 4, chatter)
        m.write(debug, bytes([dbg]))
        m.write(wavy_on, bytes([won]))
        for k, v in enumerate(ripple):
            m.w16(lvl + 0x12088 + 2 * k, v)
        for k, c in enumerate(bindings):
            m.w16(prefs + 0x10 + 2 * k, c)
        keymap[:] = bytes(16)
        for c in pressed:
            keymap[c >> 3] |= 1 << (c & 7)
        m.w32(CHARGE, 0xC0)
        m.w32(VOICE, 0xD0)
        m.w32(VOICE2, 0xD1)
        busy.clear()
        busy.update({GULP: gulp_busy, 0xD0: voice_busy, 0xD1: voice_busy})
        m.rand_seed = seed
        ev.clear()
        shots.clear()
        channel[0] = 0
        m.call("HandleKeys", sp)
        # the channels the routine kept
        kept = {m.r32(CHARGE): "c", m.r32(VOICE): "v"}
        events = []
        for e in ev:
            if e[0] == "p":
                events.append("p:%d" % e[1])
            elif e[0] == "3d":
                events.append("3d:%d:%d" % (e[1], e[2]))
            elif e[0] == "stop":
                assert e[1] == 0xC0, e
                events.append("stop")
            elif e[0] == "x":
                s16 = lambda v: ((v & 0xFFFF) ^ 0x8000) - 0x8000
                events.append("x:%d:%d:%d" % (s16(e[1]), s16(e[2]), s16(e[3])))
            elif e[0] == "b":
                events.append("b:-1" if e[1] == BEN else "b:%d" % ((e[1] - BALLOONS) // 256))
            elif e[0] == "sine":
                events.append("sine")
            elif e[0] == "shot":
                s, kind, h, v = shots[e[1]]
                assert kind == 10
                events.append("shot:%d:%d:%d:%d:%d" % (((h & 0xFFFF) ^ 0x8000) - 0x8000, ((v & 0xFFFF) ^ 0x8000) - 0x8000,
                                                       m.r16(s + 0x66), m.r16(s + 0x4E), m.r16(s + 0x50)))
        # mark the plays whose channel was kept
        n_play = 0
        for k, e in enumerate(events):
            if e.startswith("p:"):
                n_play += 1
                tag = kept.get(0x1000 + n_play)
                if tag:
                    events[k] = tag + e[1:]
        ins = ([st[n] for n, _ in TIMERS + DIRECT] + [spr[n] for n, _ in SPRITE] +
               [chatter, dbg, won] + ripple + [gulp_busy, voice_busy, seed, ",".join(map(str, bindings)),
                                               ",".join(map(str, pressed)) or "-",
                                               ",".join("%d:%d" % s for s in slots)])
        outs = ([m.r16(g[n]) for n, _ in TIMERS] + [m.r16(a) for _, a in DIRECT] + [m.r16(sp + off) for _, off in SPRITE] +
                [m.read(wavy_on, 1)[0]] + [m.r16(lvl + 0x12088 + 2 * k) for k in range(3)] +
                [",".join("%d:%d" % (m.r16(inv + 4 * i), m.r16(inv + 4 * i + 2)) for i in range(12)),
                 ",".join(events) or "-"])
        out.write(f"{' '.join(map(str, ins))} -> {' '.join(map(str, outs))}\n")
    out.close()
    print(f"keys: {CASES} cases")


if __name__ == "__main__":
    main()
