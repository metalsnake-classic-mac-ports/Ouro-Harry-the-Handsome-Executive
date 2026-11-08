"""Put a copy of the original game with its debug mode on into SheepShaver's
shared folder (the Mac's "Unix" disk), as the folder "Harry Debug": the
application patched, its data files beside it, and, when built
(re/tools/debug-switch/build.sh), the program "Harry Debug Switch" that turns
the debug mode of any copy of Harry 1.0.1 on or off on the Mac itself. The
installed game and the disks stay as they are.

The patch: `InitAppGlobals` (PPC 0x10001374) clears `gDebug` with
`li r3, 0; lwz r4, gDebug(RTOC); stb r3, 0(r4)` at 0x1000138c; the port
makes it `li r3, 1` (one byte, data fork offset 0x298f). r3 is set anew
right after, so nothing else changes. The debug mode (NOTES.md, "Cheats /
Easter eggs"): F1 comfort 144, F2 staple gun + 100 staples, F3 caffeine
pill, F4 the door key 11397, F12 comfort 0, F13 soda can, `=` an explosion
near Harry, `` ` `` held: no tile collisions; the main menu says so.

SheepShaver's shared folder on macOS takes the files' own resource forks
(`..namedfork/rsrc`) and Finder info (type, creator, flags), so they are
written as such.

Usage: python3 -I re/tools/sheepshaver_debug.py [re/work]
"""
import os
import struct
import subprocess
import sys

APP = "Harry 1.0.1.bin"
DATA_FILES = ["Harry Graphics.bin", "Harry Sounds.bin", "Harry Music.bin", "Harry Titles.bin", "Episode 1.bin"]
# li r3, 0 / lwz r4, -0x781c(r2) / stb r3, 0(r4) / lwz r5, -0x7820(r2)
CLEAR_DEBUG = bytes.fromhex("38600000808287e49864000080a287e0")
PATCHED = bytes.fromhex("38600001808287e49864000080a287e0")


def macbinary(path):
    """(name, type, creator, Finder flags, data fork, resource fork)."""
    b = open(path, "rb").read()
    name = b[2:2 + b[1]].decode("mac_roman")
    dlen, rlen = struct.unpack(">II", b[83:91])
    data = b[128:128 + dlen]
    roff = 128 + (dlen + 127) // 128 * 128
    return name, b[65:69], b[69:73], (b[73] << 8) | b[101], data, b[roff:roff + rlen]


def write_mac_file(path, ftype, creator, flags, data, rsrc):
    with open(path, "wb") as f:
        f.write(data)
    with open(path + "/..namedfork/rsrc", "wb") as f:
        f.write(rsrc)
    # FInfo (type, creator, flags without "inited", location, folder) + FXInfo
    finfo = ftype + creator + struct.pack(">H", flags & ~0x0100) + bytes(6) + bytes(16)
    subprocess.run(["xattr", "-wx", "com.apple.FinderInfo", finfo.hex(), path], check=True)


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    out = os.path.join(work, "emu-ppc", "shared", "Harry Debug")
    os.makedirs(out, exist_ok=True)
    name, ftype, creator, flags, data, rsrc = macbinary(os.path.join(work, "installed", APP))
    at = data.find(CLEAR_DEBUG)
    if at < 0 or data.find(CLEAR_DEBUG, at + 1) >= 0:
        sys.exit("the debug flag's clearing is not where it was")
    data = data[:at] + PATCHED + data[at + len(PATCHED):]
    write_mac_file(os.path.join(out, name + " (debug)"), ftype, creator, flags, data, rsrc)
    print(f"{name} (debug): patched at data fork offset {at + 3:#x}")
    for f in DATA_FILES:
        n, t, c, fl, d, r = macbinary(os.path.join(work, "installed", f))
        write_mac_file(os.path.join(out, n), t, c, fl, d, r)
        print(f"{n}: {t.decode('mac_roman')}/{c.decode('mac_roman')}")
    switch = os.path.join(work, "debug-switch", "Harry Debug Switch.bin")
    if os.path.exists(switch):
        _, t, c, fl, d, r = macbinary(switch)
        write_mac_file(os.path.join(out, "Harry Debug Switch"), t, c, fl, d, r)
        print("Harry Debug Switch: APPL/HDbS")
        _, t, c, fl, d, r = macbinary(os.path.join(work, "debug-switch", "README.txt.bin"))
        write_mac_file(os.path.join(out, "Harry Debug Switch README"), t, c, fl, d, r)
        print("Harry Debug Switch README: TEXT/ttxt")
    else:
        print("no Harry Debug Switch: build it with re/tools/debug-switch/build.sh")
    print("written to", out)


if __name__ == "__main__":
    main()
