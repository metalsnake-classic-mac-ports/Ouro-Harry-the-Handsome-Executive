"""Capture tables the original computes at start-up, by running its
routines in Unicorn (ppcmachine.py), and print them.

Usage: python3 -I emu_tables.py [re/work/ppc]
"""
import struct
import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402

FG_TABLE_SLOT = 0x10065ab0    # TOC slot: FG tile kind table, 64 x 20 bytes
BG_TABLE_SLOT = 0x10065aac    # TOC slot: BG tile kind table, 96 x 12 bytes


def rect(b):
    return "(%d, %d, %d, %d)" % struct.unpack(">hhhh", b)


def main():
    m = Machine(sys.argv[1] if len(sys.argv) > 1 else "re/work/ppc")
    m.call("InitTileHotRects")
    fg, bg = m.r32(FG_TABLE_SLOT), m.r32(BG_TABLE_SLOT)
    print("# InitTileHotRects; rects as (top, left, bottom, right) in the 32 x 32 tile")
    print("# foreground kind: collision rect, redraw rect")
    for k in range(64):
        e = m.read(fg + 20 * k, 20)
        print(f"fg {k:2} {rect(e[4:12])} {rect(e[12:20])}" + (f" head {e[:4].hex()}" if e[:4] != bytes(4) else ""))
    print("# background kind: collision rect")
    for k in range(96):
        e = m.read(bg + 12 * k, 12)
        print(f"bg {k:2} {rect(e[4:12])}" + (f" head {e[:4].hex()}" if e[:4] != bytes(4) else ""))


if __name__ == "__main__":
    main()
