"""Record the original's player physics routines for differential tests.

Runs the PPC routines in Unicorn (ppcmachine.py) on random states and
writes one line per case: the inputs, "->", the outputs. The Rust port
(crates/harry-original/tests/diff_physics.rs) must reproduce every line.

Usage: python3 -I emu_physics.py [re/work/ppc] [out dir, default re/work/diff]
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402

# globals (re/ghidra/globals.txt); "slot" = TOC slot holding the address
SLOT = {
    "physics": 0x100659D8, "caffeine": 0x10065AD4, "wet": 0x10065E8C,
    "scoot": 0x10065E84, "kick": 0x10065E80, "inch": 0x10065E7C, "pose": 0x10065E78,
    "contact": 0x10065E1C, "power": 0x10065E0C, "glide": 0x10065E60,
}
IMPULSE, FLOOR, SURFACE = 0x10068A96, 0x10068578, 0x1006856C
SPEED_V, SPEED_H, DIR, HURT, HOT = 0x4E, 0x50, 0x66, 0x8E, 0x06
CASES = 4000
DEFAULT_PHYSICS = (50, 70, 245, 252, 150)   # LoadLevelPhysics when level 0x12100 is 0


def set_physics(m, g, rnd):
    """Defaults in 4 of 5 cases, else random values like a custom level's."""
    ph = DEFAULT_PHYSICS if rnd.random() < 0.8 else (
        rnd.randint(1, 200), rnd.randint(1, 200), rnd.randint(150, 256), rnd.randint(150, 256), rnd.randint(50, 256))
    for i, v in enumerate(ph):
        m.w16(g["physics"] + 2 * i, v)
    return " ".join(map(str, ph))


def impulse(m, rnd, out):
    sp = m.alloc(0xA2)
    for k in range(CASES):
        mode = k % 36
        h, v = rnd.randint(-3000, 3000), rnd.randint(-3000, 3000)
        p = rnd.choice([rnd.randint(-400, 400), rnd.randint(-3000, 3000), rnd.randint(0, 300)])
        m.w16(sp + SPEED_H, h)
        m.w16(sp + SPEED_V, v)
        m.call("ApplyModedImpulse", sp, mode, p & 0xFFFF)
        out.write(f"{mode} {p} {h} {v} -> {m.r16(sp + SPEED_H)} {m.r16(sp + SPEED_V)}\n")


def maxspeed(m, rnd, out):
    sp = m.alloc(0xA2)
    for _ in range(CASES):
        lim = rnd.choice([2500, rnd.randint(1, 5000)])
        h, v = rnd.randint(-8000, 8000), rnd.randint(-8000, 8000)
        m.w16(sp + SPEED_H, h)
        m.w16(sp + SPEED_V, v)
        m.call("EnforceMaxSpeed", sp, lim)
        out.write(f"{lim} {h} {v} -> {m.r16(sp + SPEED_H)} {m.r16(sp + SPEED_V)}\n")


def glide(m, rnd, out):
    sp = m.alloc(0xA2)
    g = {k: m.glob(a) for k, a in SLOT.items()}
    for _ in range(CASES):
        hurt = rnd.choice([0, 0, 0, rnd.randint(1, 30), rnd.randint(20, 220)])
        floor = rnd.randint(0, 4)
        wet = rnd.choice([0, 0, rnd.randint(0, 200)])
        gl = rnd.choice([0, 0, rnd.randint(0, 120)])
        h, v = rnd.choice([rnd.randint(-3000, 3000), rnd.randint(-40, 40)]), rnd.choice([rnd.randint(-3000, 3000), rnd.randint(-40, 40)])
        surf = rnd.randint(0, 144)
        ph = set_physics(m, g, rnd)
        m.w16(sp + HURT, hurt)
        m.w16(FLOOR, floor)
        m.w16(g["wet"], wet)
        m.w16(g["glide"], gl)
        m.w16(SURFACE, surf)
        m.w16(sp + SPEED_H, h)
        m.w16(sp + SPEED_V, v)
        m.call("HandlePlayerGlide", sp)
        out.write(f"{ph} {hurt} {floor} {wet} {gl} {surf} {h} {v} -> "
                  f"{m.r16(sp + SPEED_H)} {m.r16(sp + SPEED_V)} {m.r16(SURFACE)}\n")


def movechair(m, rnd, out):
    sp = m.alloc(0xA2)
    g = {k: m.glob(a) for k, a in SLOT.items()}
    calls = []
    m.stub_routine("STPlayRegSound", 0, calls)
    for _ in range(CASES):
        st = {
            "hurt": rnd.choice([0, 0, 0, rnd.randint(1, 40)]),
            "scoot": rnd.choice([0, rnd.randint(0, 22)]),
            "kick": rnd.choice([0, rnd.randint(0, 22)]),
            "inch": rnd.choice([0, rnd.randint(0, 18)]),
            "pose": rnd.choice([0, 36, 72, 108]),
            "impulse": rnd.choice([0, rnd.randint(-300, 300)]),
            "caffeine": rnd.choice([0, 0, rnd.randint(1, 270)]),
            "wet": rnd.choice([0, 0, rnd.randint(1, 200)]),
            "contact": rnd.choice([0, 0, 64, 128, 192, 256]),
            "power": rnd.choice([0, 64, 128, 256]),
            "dir": rnd.randint(0, 35),
            "floor": rnd.randint(0, 4),
            "h": rnd.randint(-2500, 2500), "v": rnd.randint(-2500, 2500),
        }
        m.w16(sp + HURT, st["hurt"])
        m.w16(sp + DIR, st["dir"])
        m.w16(sp + SPEED_H, st["h"])
        m.w16(sp + SPEED_V, st["v"])
        m.write(sp + HOT, bytes.fromhex("0016001600 2a002a".replace(" ", "")))
        for k in ("scoot", "kick", "inch", "pose", "caffeine", "wet", "contact", "power"):
            m.w16(g[k], st[k])
        m.w16(IMPULSE, st["impulse"])
        m.w16(FLOOR, st["floor"])
        ph = set_physics(m, g, rnd)
        calls.clear()
        m.call("MoveChair", sp)
        ins = " ".join(str(st[k]) for k in ("hurt", "scoot", "kick", "inch", "pose", "impulse",
                                             "caffeine", "wet", "contact", "power", "dir", "floor", "h", "v"))
        hot = [m.r16(sp + HOT + 2 * i) for i in range(4)]
        outs = [m.r16(g["scoot"]), m.r16(g["kick"]), m.r16(g["inch"]), m.r16(g["pose"]),
                m.r16(IMPULSE), m.r16(g["power"]), m.r16(sp + SPEED_H), m.r16(sp + SPEED_V)] + hot
        out.write(f"{ph} {ins} -> {' '.join(map(str, outs))} {len(calls)}\n")


def main():
    ppc = sys.argv[1] if len(sys.argv) > 1 else "re/work/ppc"
    out_dir = sys.argv[2] if len(sys.argv) > 2 else "re/work/diff"
    os.makedirs(out_dir, exist_ok=True)
    for name, fn, head in (
        ("impulse", impulse, "mode p speedH speedV -> speedH speedV"),
        ("maxspeed", maxspeed, "max speedH speedV -> speedH speedV"),
        ("glide", glide, "physics[5] hurt floorKind wet glide surface speedH speedV -> speedH speedV surface"),
        ("movechair", movechair, "physics[5] hurt scootT kickT inchT pose impulse caffeine wet contact power dir floorKind "
                                 "speedH speedV -> scootT kickT inchT pose impulse power speedH speedV "
                                 "hotRect(t l b r) sounds"),
    ):
        m = Machine(ppc)
        rnd = random.Random(name)
        with open(os.path.join(out_dir, f"{name}.txt"), "w") as out:
            out.write(f"# {name}: original PPC routine in Unicorn; {head}\n")
            fn(m, rnd, out)
        print(f"{name}: {CASES} cases")


if __name__ == "__main__":
    main()
