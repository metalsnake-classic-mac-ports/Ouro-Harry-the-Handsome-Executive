"""Check harry_license.code against the original's own routine: run
`FUN_10059074` (the code from name, copies and STR# 900) in Unicorn for
many names and numbers of copies and compare. Nothing in the port uses
this (the port plays as registered); it backs the licence file that lets
the unchanged original run registered in systemless and the emulators.

Usage: python3 -I re/tools/emu_license.py [re/work]
"""
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from harry_license import code  # noqa: E402
from ppcmachine import Machine  # noqa: E402

MAKE_CODE = 0x10059074


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else 're/work'
    m = Machine(f'{work}/ppc')

    def get_ind_string():
        # STR# 900 #1 (checked: the only id the routine asks for)
        assert m.get(4) == 900 and m.get(5) == 1, (m.get(4), m.get(5))
        m.write(m.get(3), b'\x05Harry')
    m.stub('GetIndString', get_ind_string)
    rnd = random.Random(1)
    names = ['Ouro Test', '', 'a', 'Harry', 'zz Top', 'Ünïcödé', '{|}~`@[\\]^_']
    names += [''.join(chr(rnd.randrange(0x20, 0x7f)) for _ in range(rnd.randrange(0, 40))) for _ in range(400)]
    bad = n = 0
    for name in names:
        for copies in (1, 0, 2, 5, 100, 255, 1000, 32767, rnd.randrange(65536)):
            raw = name.encode('mac_roman')
            buf = m.alloc(0x100)
            m.write(buf, bytes([len(raw)]) + raw)
            p = m.call(MAKE_CODE, buf, copies, 900)
            got = m.read(p + 1, m.read(p, 1)[0]).decode()
            want = code(name, copies)
            n += 1
            if got != want:
                bad += 1
                if bad <= 10:
                    print(f'{name!r} {copies}: original {got}, ours {want}')
    print(f'license: {n} cases, {bad} different')
    sys.exit(1 if bad else 0)


if __name__ == '__main__':
    main()
