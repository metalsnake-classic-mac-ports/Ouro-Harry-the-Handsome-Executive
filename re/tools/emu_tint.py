"""Record the colour tables of BlitEncFace's modes: BuildTintTable (with
BuildTranslucTable, BuildPosterizationTable and BuildReddenTable) over the
screen's colour table (clut 8). Color2Index is stubbed: it logs the RGB
asked for and answers with the number of the question (mod 256), so the
tables show which question each entry came from; SATRand(5000) answers
k * 7919 mod 5000 for the k-th call.

Output: "q <red> <green> <blue>" per Color2Index call in order, then
"<table> <n> <256 entries, ','-joined>" for tint 0-15, transluc 0-15,
posterize 0, redden 0-7 and heat 0-15.
Usage: python3 -I emu_tint.py [re/work]
"""
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from macpal import clut8  # noqa: E402

TABLES = (("tint", 0x10065D00, 16), ("transluc", 0x10065D10, 16), ("posterize", 0x10065D14, 1),
          ("redden", 0x10065D0C, 8), ("heat", 0x10065D08, 16))


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    # GetGDevice -> GDevice (+0x16 gdPMap) -> PixMap (+0x2a pmTable) -> CTab
    ctab = struct.pack(">IHH", 0, 0x8000, 255) + b"".join(
        struct.pack(">4H", i, r * 0x101, g * 0x101, b * 0x101) for i, (r, g, b) in enumerate(clut8()))
    ct = m.new_handle(ctab)
    pixmap = bytearray(0x32)
    pixmap[0x2A:0x2E] = struct.pack(">I", ct)
    pm = m.new_handle(bytes(pixmap))
    gdev = bytearray(0x3E)
    gdev[0x16:0x1A] = struct.pack(">I", pm)
    gd = m.new_handle(bytes(gdev))
    m.stub("GetGDevice", lambda: gd)
    asked = []

    def color2index():
        asked.append(struct.unpack(">3H", m.read(m.get(3), 6)))
        return (len(asked) - 1) & 0xFF
    m.stub("Color2Index", color2index)
    calls = [0]

    def rand(_mm):
        calls[0] += 1
        return (calls[0] - 1) * 7919 % 5000
    m.stub_routine("SATRand", rand)
    m.call("BuildTintTable")
    with open(f"{work}/diff/tint.txt", "w") as out:
        for q in asked:
            out.write("q %d %d %d\n" % q)
        for name, slot, n in TABLES:
            for t in range(n):
                e = struct.unpack(">256h", m.read(m.glob(slot) + t * 0x200, 0x200))
                out.write(f"{name} {t} {','.join(map(str, e))}\n")
    print(f"tint: {len(asked)} colours, {calls[0]} random numbers")


if __name__ == "__main__":
    main()
