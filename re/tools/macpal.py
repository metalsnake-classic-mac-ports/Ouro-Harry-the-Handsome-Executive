"""The Mac's standard 8-bit color table (clut 8) - the screen palette of Harry.

SAT creates its offscreen ports with GetCTable(8) and a 4-bit inverse
table; the game sets no palette of its own (SetEntries only for gamma
fades). Usage: python3 -I macpal.py <dir with PICT_*.bin>... reports which
pictures already use clut 8 (no colour mapping when drawn).
"""
import glob
import os
import sys

RAMP = [0xEE, 0xDD, 0xBB, 0xAA, 0x88, 0x77, 0x55, 0x44, 0x22, 0x11]


def clut8():
    """Entry i: 0-214 the 6x6x6 cube from white (FF steps of 0x33), 215-224 red,
    225-234 green, 235-244 blue, 245-254 gray ramps, 255 black."""
    pal = [((5 - i // 36) * 0x33, (5 - (i // 6) % 6) * 0x33, (5 - i % 6) * 0x33) for i in range(215)]
    pal += [(v, 0, 0) for v in RAMP] + [(0, v, 0) for v in RAMP] + [(0, 0, v) for v in RAMP]
    pal += [(v, v, v) for v in RAMP] + [(0, 0, 0)]
    return pal


if __name__ == "__main__":
    _tools = os.environ.get("OURO_TOOLS") or os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "ouro-tools")
    sys.path.insert(0, os.path.join(_tools, "re"))   # the shared library (ouro-tools/re)
    from pict import decode
    C = clut8()
    S = set(C)
    for d in sys.argv[1:]:
        for f in sorted(glob.glob(os.path.join(d, "PICT_*.bin"))):
            try:
                w, h, rows, pal = decode(open(f, "rb").read())
            except ValueError:
                print(f"{f}: not 8-bit")
                continue
            used = set(b for r in rows for b in r)
            if all(pal[i] == C[i] for i in used):
                kind = "clut 8"
            elif all(pal[i] in S for i in used):
                kind = "own table, all colours in clut 8 (exact remap)"
            else:
                kind = f"own colours: {sum(1 for i in used if pal[i] not in S)} of {len(used)} not in clut 8"
            print(f"{f}: {w}x{h} {kind}")
