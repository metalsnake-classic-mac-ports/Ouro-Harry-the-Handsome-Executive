"""Record whole frames of the levels of Episode 1: the level start of
GameLoop (SetupLevelSprites, Harry and his weapon, the view), then frames
of HandleIdleSprites, HandleSprites (SATRun + CheckPlayerCollision),
FindUpperLeftCorner, the scroll origin (FUN_100251c8), CheckTileCollisions,
EraseSprites, DrawSpritesBehindTiles / DrawSpritesAboveTiles, UpdateMeters
and CheckGameEvents, with Harry's keys held at random (turning, scoot,
inch, kick, items, weapons).

Everything is real except sounds, the pixels (the SAT blit FUN_1006536c is
caught and the face's draw proc called for each of its calls, the rest of
drawing stubbed), ShowBalloon / CopierSave (answers per run), the phone /
buzzword texts and UpLevel (logged).

Each line: "<name=value ...> -> <name=value ...>"
  keys    per frame the held move keys as letters l r s i k, "-" none, ","-joined
  list    per sprite in SAT order: "<record>/<dump with ':'>", ";"-joined
  harry   per frame Harry's at "v:h", ","-joined
Usage: python3 -I emu_game.py [re/work]
Debugging: SEED=<seed> runs only that case (writing nothing) and prints
Harry per frame; with KIND=<kind> also the sprites of that kind, ADDR=<hex>
the record there, RAND=1 / TRACE=<routine,...> see trace_hooks.
"""
import os
import random
import struct
import sys

from unicorn import UC_HOOK_CODE
from unicorn.ppc_const import UC_PPC_REG_LR

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ppcmachine import Machine, DATA_END, TOC  # noqa: E402
from sprites import Faces, tvectors, dump, load_words  # noqa: E402
from emu_mask import masks  # noqa: E402
from cicn import decode as cicn_decode  # noqa: E402

SAT, VIEW, TARGET, PLAYER_SPRITE, PLAYER_H, PLAYER_V = 0x10065B70, 0x100659BC, 0x10065984, 0x10065980, 0x100659B4, 0x100659B0
WALLS, OBSTACLE, PREFS, INVENTORY, LEVEL_COUNTER, SEEN_LEVEL = 0x100658D4, 0x100658D0, 0x100659A0, 0x10065AC4, 0x100659CC, 0x10072722
COMFORT, FAVOR, SURFACE, COMFORT_SHOWN, SURFACE_SHOWN, FAVOR_SHOWN, CUR_SLOT = (0x1006856A, 0x1006856E, 0x1006856C, 0x10068570,
                                                                             0x10068572, 0x10068574, 0x1006857A)
KICK_CONTACT = 0x10065E1C
TV_DEATH = 0x10066FAC                               # HandlePlayerSpriteDeath
INITS = ("InitPlayerSprite", "InitHeldWeaponSprite", "InitBonusSprite", "InitItemSprite", "InitExitSprite", "InitDoorSprite",
         "InitCoolerSprite", "InitPlantSprite", "InitMessageSprite", "InitColliderSprite", "InitDartEnemySprite",
         "InitRampSprite", "InitEnemyShotSprite", "InitPlayerShotSprite", "InitEffectSprite", "InitTileHotRects",
         "InitPlayerKickRects", "InitBarrelSprite", "InitChairSprite", "InitStairsSprite", "InitButtonSprite",
         "InitSlidingDoorSprite", "InitCoWorkerSprite", "InitMailGuySprite", "InitRobotSprite", "InitFirePipeSprite",
         "InitGrateSprite", "InitGhostSprite", "InitTurretSprite", "InitReversorSprite", "InitBoulderSprite",
         "InitScientistSprite", "InitBlobSprite", "InitBossSprite")
BINDINGS = [0x7B, 0x7C, 0x7D, 0x7E, 0x31, 0x3B, 0x3A]
KEYS = {"l": 0x7B, "r": 0x7C, "s": 0x7D, "i": 0x7E, "k": 0x31, "e": 0x3B, "u": 0x3A, "1": 0x12, "2": 0x13, "3": 0x14,
        "4": 0x15}
PROP_FACES = {0x10065A60: 6034, 0x10065A5C: 6035, 0x10065A58: 6036, 0x10065A54: 6037,
              0x10065A40: 6051, 0x10065A3C: 6052, 0x10065A38: 6053, 0x10065A34: 6054,
              0x10065A0C: 6041, 0x10065A04: 6042, 0x10065A00: 6043, 0x100659FC: 6044}     # the fire pipes
CASES = 1500
ITEMS = [11299, 11599, 11799, 11999, 11899, 11499, 11699]
AMMO = {"staple": 0x10065AEC, "badass": 0x10065AE8, "shrapnel": 0x10065AE4}


def iter_list(m, sat):
    a = m.r32(sat + 0x78)
    while a:
        yield a
        a = m.r32(a + 0x2E)


def heap(m, x):
    """x as a record's kind if it points into the heap, else ""."""
    return m.r16(x) if 0x30000000 <= x < 0x40000000 else ""


def trace_hooks(m):
    """The debug traces: RAND=1 each Random call's caller, TRACE=<names>
    each call of these routines (r3–r6 and the caller)."""
    if os.environ.get("RAND"):
        def traced_random():
            print("  rand from", m.where(m.uc.reg_read(UC_PPC_REG_LR)))
            return m.tb_random()
        m.stub("Random", traced_random)

    def call(uc, _a, _s, name):
        r3, r4 = m.get(3), m.get(4)
        print("  call", name, hex(r3), heap(m, r3), "r4", hex(r4), heap(m, r4), "r5", m.get(5) & 0xFFFF,
              "r6", m.get(6) & 0xFFFF, "from", m.where(uc.reg_read(UC_PPC_REG_LR)))
    for name in filter(None, os.environ.get("TRACE", "").split(",")):
        a = m.addr(name)
        m.uc.hook_add(UC_HOOK_CODE, call, begin=a, end=a, user_data=name)


def main():
    debug_seed = int(os.environ.get("SEED", "0"))
    work = sys.argv[1] if len(sys.argv) > 1 else "re/work"
    m = Machine(f"{work}/ppc")
    # a sprite without a face makes CheckTileCollisions read its bounds at
    # address 0 (low memory on the Mac): a page of zeros
    m.uc.mem_map(0, 0x1000)
    faces = Faces(m, work)
    m.call(0x1005FCE8, 0, 1, 1, 0x40)               # SATConfigure as InitSAT does
    m.call(0x1005FD08, 0xA2)                        # the record size
    m.write(m.glob(0x10066254), b"\1")              # SAT's (graphics) init done
    m.stub("FreeMem", lambda: 0x1000000)

    def mask_face(mm):
        """FUN_1005f340(cicn): a mask face, its region at +0x1a."""
        rec = mm.alloc(0x40)
        w, h, mk, _, _ = cicn_decode(open(f"{work}/res/Harry_Graphics/cicn_{mm.get(3) & 0xFFFF}.bin", "rb").read())
        mm.w32(rec + 0x1A, mm.make_region({(x, y) for y in range(h) for x in range(w) if mk[y][x]}))
        return rec
    m.stub_routine(0x1005F340, mask_face)
    for f in INITS:
        m.call(f)
    for slot, hrsp in PROP_FACES.items():
        rec = m.alloc(0x1C)
        faces.ids[rec] = hrsp
        m.w32(m.glob(slot), rec)
    # the face loaders give every face a draw proc; the stubbed ones leave
    # those the game does not set at 0: a draw proc that only blits
    plain = m.alloc(8)
    m.w32(plain, m.addr("BlitEncFace"))
    m.w32(plain + 4, TOC)
    for rec in faces.ids:
        if m.r32(rec + 0x18) == 0:
            m.w32(rec + 0x18, plain)
    ms = masks(work)
    walls = m.alloc(64 * 0x2E)
    m.w32(m.glob(WALLS), walls)
    for k in range(32):
        m.w32(walls + 0x1A + k * 0x2E, m.make_region(ms[f"wall{k}"]))
    obstacle = m.alloc(0x40)
    m.w32(m.glob(OBSTACLE), obstacle)
    m.w32(obstacle + 0x1A, m.make_region(ms["obstacle"]))
    tv = tvectors(f"{work}/ppc")
    log = []
    for f in ("STPlayRegSound", "STPlay3DSound", "STPlay3DSoundPitched", "HaltTalkingSounds", 0x10056C0C,
              "GammaFadeOut", "GammaFadeIn", 0x1002549C, "AddFGRedraw", "CheckWorldCorruption", "LoadLevelTilesets",
              "InitTrackMap", "LoadLevelSounds", 0x1005F7EC, "IsPressed", "AddRectToUpdate",
              0x10024DE8, "RedrawGrid", "RedrawFGTiles", "GetScalePort", "CopyToScalePort", "CopyScalePortBack",
              "BlitEncFace", "FadeMusic", "DebugHook", "UpdateAmmo", "CopyStatRect",
              "PlotEncFace", "DrawNumItems", "FindInvItemFace", "UpdateInventory", 0x1005F738, "DrawBlackLines"):
        m.stub_routine(f, 0)
    m.stub_routine(0x10056DE8, 0)                   # nothing is playing

    def draw():
        """The drawing of a frame: WrapDrawSprite calls each face's draw proc
        (through __ptr_glue); BlitEncFace, the pixels, is stubbed."""
        m.call("DrawSpritesBehindTiles")
        m.call("DrawSpritesAboveTiles")

    def scroll():
        """FUN_100251c8(view h, view v): the scroll origin."""
        m.call(0x100251C8, m.r16(view + 2), m.r16(view))
    answers = []

    def show_balloon(mm):
        """ShowBalloon with the answer (fits or not) of the run's list, and
        what the original then does: shown - SATRand(16000) for modes 1, 3,
        5 and the sprite frozen unless mode 5; not shown - with prefs +8 = 3
        and no paging the view moves 16 px toward the sprite."""
        sp, mode = mm.get(3), mm.get(5) & 0xFFFF
        ans = answers.pop(0) if answers else 0
        log.append("balloon%d" % ans)
        if ans:
            if mode in (1, 3, 5):
                mm.tb_random()
            if sp and mode != 5:
                mm.write(sp + 0x92, b"\1")
        elif mm.r16(mm.glob(PREFS) + 8) == 3 and mm.r16(0x1007271E) == 0:
            view = mm.glob(VIEW)
            ph, pv = mm.r16(mm.glob(PLAYER_H)), mm.r16(mm.glob(PLAYER_V))
            mm.w16(view + 2, mm.r16(view + 2) + (-16 if mm.r16(sp + 4) + 32 < ph + 32 else 16))
            mm.w16(view, mm.r16(view) + (-16 if mm.r16(sp + 2) + 32 < pv + 32 else 16))
        return ans
    m.stub_routine("ShowBalloon", show_balloon)
    m.stub_routine("CopierSave", lambda mm: log.append("save") or 0)
    m.stub_routine("UpLevel", lambda mm: log.append("uplevel") or 0)
    for n in ("UnionRect", "ForeColor", "BackColor", "PaintRect", "RGBForeColor", "TextFont", "TextSize", "TextFace",
              "MoveTo", "DrawString", "NumToString", "LineTo", "FrameRect", "EraseRect", "InvertRect", "CopyBits"):
        m.stub(n, m.tb_nop)
    # kicking a locked door reads two never-set stack locals (BUGS.md):
    # mark such cases, the port cannot follow them
    stale = [False]

    def hit_player(uc, address, size, _):
        other, harry = m.get(4), m.get(3)
        if m.r16(other) in (903, 913, 923, 933) and m.r16(harry + 8) == 0 and m.r16(other + 0x66) == 0:
            stale[0] = True
    hp = m.addr("HitPlayerSprite")
    m.uc.hook_add(UC_HOOK_CODE, hit_player, begin=hp, end=hp)
    if debug_seed:
        trace_hooks(m)
    keymap = bytearray(16)
    m.stub("GetKeys", lambda: m.write(m.get(3), keymap))
    sat, view, target, prefs = m.glob(SAT), m.glob(VIEW), m.glob(TARGET), m.glob(PREFS)
    inv = m.r32(INVENTORY)
    levels = {}
    for n in range(1, 16):
        data = open(f"{work}/res/Episode_1/Hlvl_{n}.bin", "rb").read()
        start = struct.unpack(">hhh", data[0x120F4:0x120FA])
        entries = [struct.unpack(">hh", data[0x1000C + i * 16:0x10010 + i * 16]) for i in range(256) if data[0x10000 + i * 16]]
        levels[n] = (data, start, entries)
    load_words(m, work)
    # every case starts from the globals as they are now
    data_start = 0x10065800
    globals_after_init = m.read(data_start, DATA_END - data_start)
    mark = m.heap
    rnd = random.Random("game")
    # SEED=<seed> runs and prints only that case
    out = open(os.devnull if debug_seed else f"{work}/diff/game.txt", "w")
    out.write("# whole frames of Episode 1: name=value inputs -> name=value outputs\n")
    for case in range(CASES):
        m.heap = mark
        m.write(data_start, globals_after_init)
        level = rnd.choice([1, 1] + list(range(1, 16)))
        data, (start_v, start_h, start_dir), entries = levels[level]
        lvl = m.load_level(data)
        st = {"level": level, "seed": rnd.randint(1, 0x7FFFFFFE), "scroll": rnd.choice([0, 1, 2, 2, 3, 4]),
              "comfort": rnd.choice([144, 100, 20]), "favor": rnd.choice([144, 60]), "frames": rnd.choice([rnd.randint(1, 60), rnd.randint(1, 300)]), "h": 0, "v": 0, "dir": 0,
              "inv": "-", "curslot": -1, "staple": 0, "badass": 0, "shrapnel": 0,
              "balloons": "".join(rnd.choice("0001") for _ in range(16))}
        if rnd.random() < 0.6:
            items = rnd.sample(ITEMS, rnd.randint(1, 4))
            st["inv"] = ",".join(f"{i}:{rnd.randint(1, 3)}" for i in items)
            st["curslot"] = rnd.randint(0, len(items) - 1)
            for k in AMMO:
                st[k] = rnd.choice([0, 3, 50, 100])
        # Harry at the level's start or next to one of its sprites
        if rnd.random() < 0.3:
            st["h"], st["v"], st["dir"] = start_h - 32, start_v - 32, start_dir
        else:
            e = rnd.choice(entries)
            st["h"], st["v"] = e[1] - 6 + rnd.randint(-70, 40), e[0] - 6 + rnd.randint(-70, 40)
            st["dir"] = rnd.randint(0, 35)
        held, keys = "", []
        for _f in range(st["frames"]):
            if rnd.random() < 0.15:
                pool = "llrrsssssiikk" + ("uuuue1234" if st["inv"] != "-" else "")
                held = "".join(sorted(set(rnd.sample(pool, rnd.choice([0, 1, 1, 1, 2])))))
            keys.append(held or "-")
        st["keys"] = ",".join(keys)
        if debug_seed and debug_seed != st["seed"]:
            continue
        m.w32(sat + 0x78, 0)
        m.w32(sat + 0x110, 0)
        m.write(prefs, bytes(0x20))
        m.w16(prefs + 8, st["scroll"])
        for i, b in enumerate(BINDINGS):
            m.w16(prefs + 0x10 + 2 * i, b)
        slots = [tuple(map(int, x.split(":"))) for x in st["inv"].split(",")] if st["inv"] != "-" else []
        slots += [(-1, 0)] * (12 - len(slots))
        for i, (oid, cnt) in enumerate(slots):
            m.w16(inv + 4 * i, oid)
            m.w16(inv + 4 * i + 2, cnt)
        m.w16(CUR_SLOT, st["curslot"])
        for k, slot in AMMO.items():
            m.w16(m.glob(slot), st[k])
        for a, v in ((COMFORT, st["comfort"]), (COMFORT_SHOWN, st["comfort"]), (FAVOR, st["favor"]),
                     (FAVOR_SHOWN, st["favor"]), (SURFACE, 75), (SURFACE_SHOWN, 75)):
            m.w16(a, v)
        m.w16(m.glob(LEVEL_COUNTER), level)
        m.w16(SEEN_LEVEL, level)
        m.rand_seed = st["seed"]
        log.clear()
        stale[0] = False
        answers[:] = [int(c) for c in st["balloons"]]
        # the level start of GameLoop
        m.call("SetupLevelSprites")
        h, v = st["h"], st["v"]
        p = m.call("SATNewSprite", 0, h, v, 0x10066FB4)
        m.w32(m.glob(PLAYER_SPRITE), p)
        m.w16(p + 0x66, st["dir"])
        m.w16(m.glob(PLAYER_H), h - 32)
        m.w16(m.glob(PLAYER_V), v - 32)
        m.write(target, struct.pack(">hh", v - 208, h - 208))
        m.write(view, struct.pack(">hh", v - 208, h - 208))
        m.call("SATNewSprite", 0, h, v, 0x1006724C)
        m.call("FindUpperLeftCorner")
        scroll()
        harry = []
        if debug_seed:
            print("start", dump(m, p, faces, tv), "rand", m.rand_seed)
        for f in range(st["frames"]):
            keymap[:] = bytes(16)
            for c in keys[f]:
                if c != "-":
                    code = KEYS[c]
                    keymap[code >> 3] |= 1 << (code & 7)
            m.call("HandleIdleSprites")
            m.call("HandleSprites")
            m.call("FindUpperLeftCorner")
            scroll()
            m.call("CheckTileCollisions")
            m.call("EraseSprites")
            draw()
            m.call("UpdateMeters", 1)
            m.call("CheckGameEvents")
            harry.append("%d:%d" % (m.r16(p + 0x5A), m.r16(p + 0x5C)))
            if debug_seed == st["seed"]:
                print(f, keys[f], dump(m, p, faces, tv), "comfort", m.r16(COMFORT), "slot", m.r16(CUR_SLOT), "rand", m.rand_seed,
                      "log", log[-3:], "list", ",".join(str(m.r16(a)) for a in iter_list(m, sat)))
                if os.environ.get("ADDR"):
                    print("   @", dump(m, int(os.environ["ADDR"], 16), faces, tv))
                for a in iter_list(m, sat):
                    if m.r16(a) == int(os.environ.get("KIND", "-1")):
                        print("   ", dump(m, a, faces, tv))
            if "uplevel" in log:
                break
            if m.r32(p + 0x1A) == TV_DEATH:
                log.append("died")
                break
        lst, a = [], m.r32(sat + 0x78)
        while a:
            lst.append(a)
            a = m.r32(a + 0x2E)
        free, a = [], m.r32(sat + 0x110)
        while a:
            free.append(a)
            a = m.r32(a + 0x2E)
        ids = {a: n for n, a in enumerate(sorted(lst + free))}
        o = {"frames": len(harry), "seed": m.rand_seed, "view": "%d:%d" % struct.unpack(">hh", m.read(view, 4)),
             "target": "%d:%d" % struct.unpack(">hh", m.read(target, 4)),
             "meters": ":".join(str(m.r16(x)) for x in (COMFORT, COMFORT_SHOWN, SURFACE, SURFACE_SHOWN, FAVOR, FAVOR_SHOWN)),
             "contact": m.r16(m.glob(KICK_CONTACT)), "counter": m.r16(m.glob(LEVEL_COUNTER)),
             "log": ",".join(log) or "-", "staledoor": int(stale[0]),
             "inv": ",".join("%d:%d" % (m.r16(inv + 4 * i), m.r16(inv + 4 * i + 2)) for i in range(12)),
             "curslot": m.r16(CUR_SLOT), "ammo": ":".join(str(m.r16(m.glob(a))) for a in AMMO.values()), "harry": ",".join(harry),
             "list": ";".join(f"{ids[a]}/{dump(m, a, faces, tv).replace(' ', ':')}" for a in lst),
             "free": ",".join(str(ids[a]) for a in free) or "-"}
        out.write(" ".join(f"{k}={v}" for k, v in st.items()) + " -> " + " ".join(f"{k}={v}" for k, v in o.items()) + "\n")
    out.close()
    print(f"game: {CASES} cases")


if __name__ == "__main__":
    main()
