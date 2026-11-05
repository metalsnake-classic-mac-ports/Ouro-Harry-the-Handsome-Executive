"""Read the original's globals in systemless after a scripted run from an
`ouro-harry-repro` save.

    python3 -I re/tools/sl_peek.py SAVE [STEPS...] [--read NAME=ADDR ...]

STEPS as in sl_repro.py (keys held N ticks, wait:N; no screenshots). After
them the run pauses and each global is read: ADDR is its address in the
Ghidra project (re/ghidra/globals.txt); a leading @ means the TOC slot
holds a pointer to it. Default: the three ammunition counters and the
meters.

It needs a systemless built with the debugger socket (the Homebrew build
has none):
    cargo install systemless --version 0.109.0 --locked \\
        --features debug-server --root ../ouro-tools/systemless
found as $SYSTEMLESS or ../ouro-tools/systemless/bin/systemless. The
game goes in as a zip (the socket mode cannot take a folder), the save
as MacBinary in it. The data section is found by a text it holds (the
B-E-N message at +0x3718) and the reads are relative to it, as in Ghidra
(data at 0x10065390).
"""
import importlib.util
import json
import os
import pathlib
import socket
import struct
import subprocess
import sys
import time
import zipfile

HERE = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('sl_repro', HERE / 'sl_repro.py')
sl = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sl)

GHIDRA_DATA = 0x10065390
DEFAULT = {'staple': '@10065aec', 'badass': '@10065ae8', 'shrapnel': '@10065ae4',
           'comfort': '1006856a', 'surface': '1006856c', 'favor': '1006856e'}
SOCK = '/tmp/ouro-harry-sl.sock'


def crc16(b):
    c = 0
    for x in b:
        c ^= x << 8
        for _ in range(8):
            c = ((c << 1) ^ 0x1021) & 0xffff if c & 0x8000 else (c << 1) & 0xffff
    return c


def macbinary(name, rsrc, finder_info):
    h = bytearray(128)
    n = name.encode('mac_roman')
    h[1] = len(n)
    h[2:2 + len(n)] = n
    h[65:73] = finder_info[:8]
    struct.pack_into('>II', h, 83, 0, len(rsrc))
    h[122] = h[123] = 129
    struct.pack_into('>H', h, 124, crc16(h[:124]))
    return bytes(h) + rsrc + bytes(-len(rsrc) % 128)


def pack(game, out):
    with zipfile.ZipFile(out, 'w') as z:
        for f in sorted(os.listdir(game)):
            p = game / f
            if f.endswith('.bin'):
                z.write(p, f)
                continue
            rsrc = (p / '..namedfork/rsrc').read_bytes()
            info = subprocess.run(['xattr', '-px', 'com.apple.FinderInfo', str(p)], capture_output=True, text=True, check=True)
            z.writestr(f + '.bin', macbinary(f, rsrc, bytes.fromhex(''.join(info.stdout.split()))))


def inputs(steps):
    """sl_repro's script as `--tick-input-script` lines; the end tick."""
    t, lines = 0, []
    for a in sl.script(steps)['actions']:
        kind = a['type']
        if kind == 'run':
            t += a['ticks']
        elif kind in ('mouse_down', 'mouse_up'):
            lines.append(f"{t} {kind.replace('_', '')} {a['v']} {a['h']}")
        elif kind in ('key_down', 'key_up'):
            lines.append(f"{t} {kind.replace('_', '')} {a['key']} {a['ch']}")
    return lines, t


def main():
    args = sys.argv[1:]
    reads = dict(DEFAULT)
    if '--read' in args:
        i = args.index('--read')
        reads = dict(r.split('=', 1) for r in args[i + 1:])
        del args[i:]
    if not args:
        sys.exit(__doc__)
    save, steps = args[0], [s for s in args[1:] if not s.startswith(('shot:', 'frames:'))]
    work = sl.WORK / 'systemless'
    zip_path = work / f'{save}.zip'
    pack(sl.build(save), zip_path)
    lines, end = inputs(steps)
    script = work / f'{save}-input.txt'
    script.write_text('\n'.join(lines) + '\n')
    exe = os.environ.get('SYSTEMLESS') or str(sl.ROOT.parent / 'ouro-tools/systemless/bin/systemless')
    if os.path.exists(SOCK):
        os.remove(SOCK)
    log = open(work / f'{save}-peek.log', 'w')
    proc = subprocess.Popen([exe, str(zip_path), '--headless', '--screen-depth', '8', '--max-ticks', str(end + 600),
                             '--tick-input-script', str(script), '--debug-socket', SOCK], stdout=log, stderr=subprocess.STDOUT)
    try:
        for _ in range(300):
            if os.path.exists(SOCK):
                break
            time.sleep(0.1)
        c = socket.socket(socket.AF_UNIX)
        c.connect(SOCK)
        f = c.makefile('rw')

        def req(r):
            f.write(json.dumps({'id': 1, 'request': r}) + '\n')
            f.flush()
            return json.loads(f.readline())

        def mem(a, n):
            r = req({'op': 'read_memory', 'space': 2, 'address': a, 'length': n})
            return bytes(r['reply']['value']['bytes']) if 'reply' in r else b''

        req({'op': 'resume'})
        while True:
            st = json.dumps(req({'op': 'execution_status'}))
            ticks = [int(x) for x in __import__('re').findall(r'"guest_tick":\s*(\d+)', st)]
            if ticks and max(ticks) >= end + 60:
                break
            time.sleep(0.05)
        req({'op': 'pause'})
        # the data section: where the B-E-N text (Pascal string at +0x3718) is
        base = None
        pat = b'Ben? You mean Ben Spees?'
        for a in [0x2000000] + list(range(0, 0x8000000, 0x100000)):
            k = mem(a, 0x100040).find(pat)
            if k >= 0 and mem(a + k - 0x3719 + 0x3819, 12) == b'Heh heh heh.':
                base = a + k - 0x3719
                break
        if base is None:
            sys.exit('data section not found')
        print(f'{save}: after tick {end + 60}, data section at {base:#x}')
        for name, addr in reads.items():
            at = base + int(addr.lstrip('@'), 16) - GHIDRA_DATA
            if addr.startswith('@'):
                at = struct.unpack('>I', mem(at, 4))[0]
            print(f'  {name} = {struct.unpack(">h", mem(at, 2))[0]}')
    finally:
        proc.terminate()


if __name__ == '__main__':
    main()
