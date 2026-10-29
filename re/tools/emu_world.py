"""Record the level start and the sprite bookkeeping: SetupLevelSprites on
a level whose sprite list is cut down to the kinds the port has, then
frames of HandleIdleSprites (the view moving), random deaths (a sprite of
the list frozen, or its task cleared) and EraseSprites (its drawing
stubbed). No sprite tasks run.

Each line: "<name=value ...> -> list=<..> free=<..> idle=<..> level=<..> props=<..>"
  list   per sprite in SAT order: "<record>/<dump with ':' for ' '>", ";"-joined
         (records numbered by address = the order they were made)
  free   the free list, the next record to be reused first
  idle   used slots: "<slot>:<active record or -1>:<kind>:<at v>:<at h>:..."
  level  the entries that changed: "<index>:<present>:<id>:<v>:<h>"
  props  background props "v:h:face", then "|", the foreground ones
Usage: python3 -I emu_world.py [re/work]
"""
import os
import random
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine  # noqa: E402
from sprites import Faces, tvectors, dump  # noqa: E402

SAT, VIEW, IDLE = 0x10065B70, 0x100659BC, 0x1006DCF0
BG_PROPS, FG_PROPS = 0x100723AC, 0x100720AC
INITS = ("InitBonusSprite", "InitItemSprite", "InitExitSprite", "InitDoorSprite", "InitCoolerSprite",
         "InitPlantSprite", "InitMessageSprite", "InitColliderSprite", "InitDartEnemySprite", "InitRampSprite",
         "InitEnemyShotSprite", "InitBarrelSprite", "InitChairSprite", "InitStairsSprite", "InitButtonSprite",
         "InitSlidingDoorSprite", "InitCoWorkerSprite", "InitMailGuySprite", "InitRobotSprite", "InitFirePipeSprite",
         "InitEffectSprite")
# the props' faces as InitFaces loads them (its other faces need more Toolbox)
PROP_FACES = {0x10065A60: 6034, 0x10065A5C: 6035, 0x10065A58: 6036, 0x10065A54: 6037,
              0x10065A40: 6051, 0x10065A3C: 6052, 0x10065A38: 6053, 0x10065A34: 6054,
              0x10065A0C: 6041, 0x10065A04: 6042, 0x10065A00: 6043, 0x100659FC: 6044}     # the fire pipes
CASES = 3000


def ported(k):
    return (k in (500, 501, 601) or 700 <= k <= 799 or 900 <= k <= 949 or 1000 <= k <= 1049 or k in (1609, 1619, 1629, 1639)
            or 2301 <= k <= 2304 or k in (3000, 3100, 3101, 9199, 9200, 9300, 9600, 9800) or 9401 <= k <= 9499
            or 9700 <= k <= 9799 or 9990 <= k <= 9999 or 11001 <= k <= 19999 or k >= 21000
            or k in (550, 560, 1601, 1611, 1621, 1631) or 950 <= k <= 999 or 1300 <= k <= 1449 or 1800 <= k <= 1849
            or 2000 <= k <= 2099 or 4200 <= k <= 4299 or 9500 <= k <= 9599 or 9900 <= k <= 9909)


def s16(v):
    return ((v & 0xFFFF) ^ 0x8000) - 0x8000


def main():
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    faces = Faces(m, work)
    m.call(0x1005FCE8, 0, 1, 1, 0x40)               # SATConfigure as InitSAT does
    m.call(0x1005FD08, 0xA2)                        # the record size, as InitSAT sets it
    m.write(m.glob(0x10066254), b"\1")              # SAT's (graphics) init done
    m.stub_routine(0x1005F340, lambda mm: mm.alloc(0x40))
    for f in INITS:
        m.call(f)
    for slot, hrsp in PROP_FACES.items():
        rec = m.alloc(0x1C)
        faces.ids[rec] = hrsp
        m.w32(m.glob(slot), rec)
    tv = tvectors(f"{work}/ppc")
    for f in ("CheckWorldCorruption", "LoadLevelTilesets", 0x1002549C, "InitTrackMap", "LoadLevelSounds", 0x1005F7EC,
              "IsPressed", "AddRectToUpdate", "FindScaledPosAndBounds", 0x10024DE8):
        m.stub_routine(f, 0)
    m.stub("UnionRect", m.tb_nop)
    m.stub("FreeMem", lambda: 0x1000000)            # SAT asks before each new record
    sat = m.glob(SAT)
    view = m.glob(VIEW)
    datas = [open(f"{work}/res/Episode_1/Hlvl_{n}.bin", "rb").read() for n in range(1, 16)]
    mark = m.heap
    rnd = random.Random("world")
    fmt = lambda d: " ".join(f"{k}={v}" for k, v in d.items())
    out = open(f"{work}/diff/world.txt", "w")
    out.write("# level start, idle sprites, EraseSprites: name=value inputs -> state\n")
    for case in range(CASES):
        m.heap = mark
        lv = rnd.choice([1, 1, 1] + list(range(2, 16)))
        data = bytearray(datas[lv - 1])
        for i in range(256):
            if data[0x10000 + i * 16] and not ported(struct.unpack(">h", data[0x10002 + i * 16:0x10004 + i * 16])[0]):
                data[0x10000 + i * 16] = 0
        lvl = m.load_level(bytes(data))
        start = struct.unpack(">hh", data[0x120F4:0x120F8])          # (v, h) of the player's start
        st = {"level": lv, "seed": rnd.randint(1, 0x7FFFFFFE), "frames": rnd.choice([0, 1, 2, rnd.randint(0, 30)])}
        vv, vh = start[0] - 208 + rnd.randint(-100, 100), start[1] - 208 + rnd.randint(-100, 100)
        steps = []
        for _f in range(st["frames"]):
            if rnd.random() < 0.1:
                vv, vh = rnd.randint(-200, 3900), rnd.randint(-200, 3900)
            else:
                vv, vh = vv + rnd.randint(-40, 40), vh + rnd.randint(-40, 40)
            kill = "-"
            if rnd.random() < 0.3:
                kill = f"{rnd.randint(0, 30)}:{rnd.choice(['f', 't'])}"
            steps.append(f"{vv}:{vh}:{kill}")
        st["steps"] = ",".join(steps) or "-"
        m.rand_seed = st["seed"]
        # a fresh SAT: no sprites, no free records
        m.w32(sat + 0x78, 0)
        m.w32(sat + 0x110, 0)
        m.call("SetupLevelSprites")
        before = bytes(m.read(lvl + 0x10000, 0x1000))
        for step in steps:
            a, b, k = step.split(":", 2)
            m.write(view, struct.pack(">hh", int(a), int(b)))
            m.call("HandleIdleSprites")
            if k != "-":
                pos, mode = k.split(":")
                p = m.r32(sat + 0x78)
                for _ in range(int(pos)):
                    if p:
                        p = m.r32(p + 0x2E)
                if p:
                    if mode == "f":
                        m.write(p + 0x92, b"\1")
                    else:
                        m.w32(p + 0x1A, 0)
            m.call("EraseSprites")
        # records: in the list and on the free list, numbered by address
        lst, p = [], m.r32(sat + 0x78)
        while p:
            lst.append(p)
            p = m.r32(p + 0x2E)
        free, p = [], m.r32(sat + 0x110)
        while p:
            free.append(p)
            p = m.r32(p + 0x2E)
        ids = {a: n for n, a in enumerate(sorted(lst + free))}
        o = {"list": ";".join(f"{ids[a]}/{dump(m, a, faces, tv).replace(' ', ':')}" for a in lst) or "-",
             "free": ",".join(str(ids[a]) for a in free) or "-"}
        idle = []
        for slot in range(255):
            b = IDLE + slot * 0x44
            if m.read(b, 1)[0]:
                act = m.r32(b + 4)
                r16 = lambda off: m.r16(b + off)
                idle.append(":".join(map(str, (slot, ids.get(act, -1) if act else -1, faces.name(m.r32(b + 8)),
                                                m.read(b + 0xC, 1)[0], r16(0xE), r16(0x10), r16(0x12), r16(0x16),
                                                r16(0x14), r16(0x18), r16(0x1A), r16(0x1C), r16(0x1E), r16(0x20),
                                                r16(0x22), r16(0x24), r16(0x26), r16(0x28), r16(0x2A),
                                                struct.unpack(">i", m.read(b + 0x2C, 4))[0], m.read(b + 0x30, 1)[0],
                                                m.read(b + 0x31, 1)[0], r16(0x32), r16(0x34), r16(0x36),
                                                tv.get(m.r32(b + 0x38), "?"), tv.get(m.r32(b + 0x3C), "?"),
                                                tv.get(m.r32(b + 0x40), "?")))))
        o["idle"] = ";".join(idle) or "-"
        after = m.read(lvl + 0x10000, 0x1000)
        lev = []
        for i in range(256):
            if after[i * 16:i * 16 + 16] != before[i * 16:i * 16 + 16]:
                e = after[i * 16:i * 16 + 16]
                lev.append("%d:%d:%d:%d:%d" % ((i, e[0]) + struct.unpack(">h", e[2:4]) + struct.unpack(">hh", e[12:16])))
        o["level"] = ";".join(lev) or "-"
        props = []
        for base in (BG_PROPS, FG_PROPS):
            one = []
            for i in range(64):
                b = base + i * 12
                if not m.read(b, 1)[0]:
                    break
                v, h = struct.unpack(">hh", m.read(b + 2, 4))
                one.append(f"{v}:{h}:{faces.name(m.r32(b + 8))}")
            props.append(",".join(one) or "-")
        o["props"] = "|".join(props)
        out.write(f"{fmt(st)} -> {fmt(o)}\n")
    out.close()
    print(f"world: {CASES} cases")


if __name__ == "__main__":
    main()
