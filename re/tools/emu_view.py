"""Record FindUpperLeftCorner: a random walk of Harry (gPlayerH / V) over
frames, with the scroll preference (prefs +8, 0-4), prefs +2 / +3 and the
shake counter as inputs.

Each line: "<name=value ...> -> <per frame: v:h:tv:th:steps:settle:dir:shake:area:full:black, ';'-joined>"
Usage: python3 -I emu_view.py [re/work]
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402

VIEW, TARGET, SHAKE, PREFS, PLAYER_H, PLAYER_V = 0x100659BC, 0x10065984, 0x10065AD0, 0x100659A0, 0x100659B4, 0x100659B0
STEPS, SETTLE, DIR, AREA, FULL = 0x1007271E, 0x1007271C, 0x10072720, 0x10072710, 0x10065974
CASES = 6000


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    black = []
    m.stub_routine("DrawBlackLines", lambda mm: black.append(1) or 0)
    view, target, shake, prefs = m.glob(VIEW), m.glob(TARGET), m.glob(SHAKE), m.glob(PREFS)
    ph_, pv_, full = m.glob(PLAYER_H), m.glob(PLAYER_V), m.glob(FULL)
    rnd = random.Random("view")
    out = open(f"{work}/diff/view.txt", "w")
    out.write("# FindUpperLeftCorner: name=value inputs -> frames\n")
    for _ in range(CASES):
        ph, pv = rnd.randint(0, 3900), rnd.randint(0, 3900)
        st = {"mode": rnd.choice([0, 1, 2, 2, 3, 4]), "p2": rnd.choice([0, 1]), "p3": rnd.choice([0, 1]),
              "vv": pv - 208 + rnd.choice([0, rnd.randint(-300, 300)]), "vh": ph - 208 + rnd.choice([0, rnd.randint(-300, 300)]),
              "tv": pv - 208 + rnd.randint(-100, 100), "th": ph - 208 + rnd.randint(-100, 100),
              "steps": rnd.choice([0, 0, 0, 1, 5, 12]), "settle": rnd.choice([0, 1, 2]), "dir": rnd.randint(1, 8),
              "shake": rnd.choice([0, 0, 0, rnd.randint(1, 30)])}
        path = []
        for _f in range(rnd.choice([1, 2, rnd.randint(1, 40)])):
            if rnd.random() < 0.05:
                ph, pv = rnd.randint(0, 3900), rnd.randint(0, 3900)
            else:
                ph, pv = ph + rnd.randint(-12, 12), pv + rnd.randint(-12, 12)
            path.append(f"{ph}:{pv}")
        if st["mode"] >= 3 and rnd.random() < 0.4:
            # Harry at the 73 px line across, past the 144 px one along
            a, b = rnd.choice([-1, 1]) * rnd.randint(72, 73), rnd.choice([-1, 1]) * rnd.randint(145, 160)
            if rnd.random() < 0.5:
                a, b = b, a
            st["steps"] = 0
            path[0] = f"{st['vh'] + 224 + a - 32}:{st['vv'] + 224 + b - 32}"
        st["path"] = ",".join(path)
        m.w16(prefs + 8, st["mode"])
        m.write(prefs + 2, bytes([st["p2"], st["p3"]]))
        m.w16(view, st["vv"])
        m.w16(view + 2, st["vh"])
        m.w16(target, st["tv"])
        m.w16(target + 2, st["th"])
        m.w16(STEPS, st["steps"])
        m.w16(SETTLE, st["settle"])
        m.w16(DIR, st["dir"])
        m.w16(shake, st["shake"])
        frames = []
        for p in path:
            h, v = map(int, p.split(":"))
            m.w16(ph_, h)
            m.w16(pv_, v)
            m.write(AREA, b"\0")
            m.write(full, b"\0")
            black.clear()
            m.call("FindUpperLeftCorner")
            frames.append(":".join(map(str, (m.r16(view), m.r16(view + 2), m.r16(target), m.r16(target + 2), m.r16(STEPS),
                                             m.r16(SETTLE), m.r16(DIR), m.r16(shake), m.read(AREA, 1)[0],
                                             m.read(full, 1)[0], len(black)))))
        out.write(" ".join(f"{k}={v}" for k, v in st.items()) + " -> " + ";".join(frames) + "\n")
    out.close()
    print(f"view: {CASES} cases")


if __name__ == "__main__":
    main()
