"""Harry's registration: the code its check expects, and the licence file.

    python3 -I re/tools/harry_license.py NAME [COPIES] [--write DIR]

The check (`FUN_10057cb0`, NOTES.md "Registration") reads "Harry License"
(type 'Areg', creator 'Alic', written by the Register Harry utility) from
the Preferences folder: 0x202 bytes, the name as a Pascal string at 0,
the code at 0x100, the number of copies (big-endian word) at 0x200. It
computes the code from the name and the product name (STR# 900 #1,
"Harry") and compares it case-sensitively (`EqualString`).

The code, as `FUN_10059074` computes it: the name upper-cased (ASCII
a-z only, `FUN_10058f9c`); a 32-bit sum starting at 0; for each byte of
the name, then of "Harry" (`FUN_10058fd8`): low byte += byte, rotate left
5, low word += copies; then eight letters, each 'A' + the low nibble,
rotating right 4 after each.

--write DIR puts the file at DIR/System Folder/Preferences/Harry License
(systemless's Preferences folder when DIR is the game folder), with type
and creator in the Finder info (macOS).
"""
import pathlib
import struct
import subprocess
import sys

PRODUCT = b'Harry'
M32 = 0xffffffff


def rotl(x, n):
    return ((x << n) | (x >> (32 - n))) & M32


def mix(seed, s, copies):
    for b in s:
        seed = (seed & 0xffffff00) | ((seed + b) & 0xff)
        seed = rotl(seed, 5)
        seed = (seed & 0xffff0000) | ((seed + copies) & 0xffff)
    return seed


def code(name, copies=1):
    up = bytes(b - 0x20 if 0x60 < b < 0x7b else b for b in name.encode('mac_roman'))
    seed = mix(mix(0, up, copies), PRODUCT, copies)
    out = []
    for _ in range(8):
        out.append(chr((seed & 0xf) + 0x41))
        seed = rotl(seed, 28)
    return ''.join(out)


def licence(name, copies=1):
    d = bytearray(0x202)
    n, c = name.encode('mac_roman'), code(name, copies).encode()
    d[0] = len(n)
    d[1:1 + len(n)] = n
    d[0x100] = len(c)
    d[0x101:0x101 + len(c)] = c
    struct.pack_into('>H', d, 0x200, copies)
    return bytes(d)


def write(folder, name, copies=1):
    prefs = pathlib.Path(folder) / 'System Folder' / 'Preferences'
    prefs.mkdir(parents=True, exist_ok=True)
    path = prefs / 'Harry License'
    path.write_bytes(licence(name, copies))
    info = (b'AregAlic' + bytes(24)).hex()
    subprocess.run(['xattr', '-wx', 'com.apple.FinderInfo', info, str(path)], check=True)
    return path


def main():
    args = sys.argv[1:]
    folder = None
    if '--write' in args:
        i = args.index('--write')
        folder = args[i + 1]
        del args[i:i + 2]
    if not args:
        sys.exit(__doc__)
    name, copies = args[0], int(args[1]) if len(args) > 1 else 1
    print(code(name, copies))
    if folder:
        print(write(folder, name, copies))


if __name__ == '__main__':
    main()
