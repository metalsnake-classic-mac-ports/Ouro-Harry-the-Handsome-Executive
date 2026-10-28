"""Record co-workers (1300-1449): SetupCoWorkerSprite, then frames of
HandleCoWorkerSprite (1301's animation with SATRand), some starting on
layer 10 (hit by a shot).

Each line: "<name=value ...> -> <dump>"
Usage: python3 -I emu_coworker.py [re/work]
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from sprites import Faces, tvectors, dump  # noqa: E402

KINDS = [1301] * 6 + [1300, 1311, 1321, 1341, 1361, 1381, 1401, 1421, 1441, 1449]
CASES = 15000


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m)
    m.call("InitCoWorkerSprite")
    tv = tvectors(f"{work}/ppc")
    sp = m.alloc(0xA2)
    rnd = random.Random("coworker")
    fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())
    out = open(f"{work}/diff/coworker.txt", "w")
    out.write("# co-workers: name=value inputs -> dump\n")
    for _ in range(CASES):
        st = {"kind": rnd.choice(KINDS), "v": rnd.randint(200, 3800), "h": rnd.randint(200, 3800),
              "dir": rnd.choice([0, 0, rnd.randint(-2, 34), 11, 12, 31, 32]), "layer": rnd.choice([5, 5, 10]),
              "frozen": int(rnd.random() < 0.03), "frames": rnd.randint(1, 60), "seed": rnd.randint(1, 0x7FFFFFFE)}
        m.write(sp, bytes(0xA2))
        m.w16(sp, st["kind"])
        m.w16(sp + 2, st["v"])
        m.w16(sp + 4, st["h"])
        m.call("SetupCoWorkerSprite", sp)
        m.w16(sp + 0x66, st["dir"])
        m.w16(sp + 0x4C, st["layer"])
        m.write(sp + 0x92, bytes([st["frozen"]]))
        m.rand_seed = st["seed"]
        for _f in range(st["frames"]):
            m.call("HandleCoWorkerSprite", sp)
        out.write(f"{fmt(st)} -> {dump(m, sp, faces, tv)}\n")
    out.close()
    print(f"coworker: {CASES} cases")


if __name__ == "__main__":
    main()
