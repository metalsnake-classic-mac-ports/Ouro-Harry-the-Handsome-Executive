"""Record BuildSineTable (the ripple's table) for every length 0-120 (it
cuts it to 100) and widths from -20 to 40.

Each line: "<length> <width> -> <length after> <entries, ','-joined>"
Usage: python3 -I emu_sine.py [re/work]
"""
import math
import os
import struct
import sys

from unicorn.ppc_const import UC_PPC_REG_FPR1

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")

    def sine():
        x = struct.unpack(">d", m.uc.reg_read(UC_PPC_REG_FPR1).to_bytes(8, "big"))[0]
        m.uc.reg_write(UC_PPC_REG_FPR1, int.from_bytes(struct.pack(">d", math.sin(x)), "big"))
    m.stub("sin", sine)
    m.load_level(open(f"{work}/res/Episode_1/Hlvl_10.bin", "rb").read())
    lvl = m.r32(m.r32(m.glob(0x10065B64)))
    table = m.glob(0x10065D04)
    out = open(f"{work}/diff/sine.txt", "w")
    n = 0
    for length in range(0, 121):
        for width in list(range(-20, 41)) + [100, 1000, -1000, 32767]:
            m.write(lvl + 0x12088, struct.pack(">hh", length, width))
            m.write(table, bytes(200))
            m.call("BuildSineTable")
            after = struct.unpack(">h", m.read(lvl + 0x12088, 2))[0]
            entries = struct.unpack(">%dh" % after, m.read(table, 2 * after))
            out.write(f"{length} {width} -> {after} {','.join(map(str, entries)) or '-'}\n")
            n += 1
    out.close()
    print(f"sine: {n} cases")


if __name__ == "__main__":
    main()
