"""Record whole frames of the levels of Episode 1: the level start of
GameLoop (SetupLevelSprites, Harry and his weapon, the view), then frames
of HandleIdleSprites, HandleSprites (SATRun + CheckPlayerCollision),
FindUpperLeftCorner, the scroll origin (FUN_100251c8), CheckTileCollisions,
EraseSprites, DrawSpritesBehindTiles / DrawSpritesAboveTiles, UpdateMeters
and CheckGameEvents, with Harry's keys held at random (turning, scoot,
inch, kick, items, weapons). A run ends when the level is done or the
game loop would end (Harry's death over).

Everything is real except sounds, the pixels (the SAT blit FUN_1006536c is
caught and the face's draw proc called for each of its calls, the rest of
drawing stubbed), ShowBalloon / CopierSave (answers per run), the phone /
buzzword texts, UpLevel's pictures, saving and music.

Each line: "<name=value ...> -> <name=value ...>"
  keys    per frame the held move keys as letters l r s i k, "-" none, ","-joined
  list    per sprite in SAT order: "<record>/<dump with ':'>", ";"-joined
  harry   per frame Harry's at "v:h", ","-joined
  meterdraw  per meter what UpdateMeters drew last: "<strip length>:<end>"
  blits   the draw procs' BlitEncFace calls "<face>:<mode>:<arg>;", each
          frame ended by "|": their number and CRC-32
Usage: python3 -I emu_game.py [re/work]
Debugging: SEED=<seed> runs only that case (writing nothing) and prints
Harry per frame; with KIND=<kind> also the sprites of that kind, ADDR=<hex>
the record there, BLITS=1 the draw procs' blits, RAND=1 /
TRACE=<routine,...> see trace_hooks.
"""
import math
import os
import random
import struct
import sys
import zlib

from unicorn import UC_HOOK_CODE
from unicorn.ppc_const import UC_PPC_REG_FPR1, UC_PPC_REG_LR

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
SCORE = 0x10065AF8
# the state at the level's start (UpLevel): comfort, surface, favor (shorts),
# score (long), staple / B.A.D.A.S.S. / shrapnel ammo (shorts); the
# inventory copy
START_SHORTS, START_SCORE, START_AMMO, START_INV = (0x10065B1C, 0x10065B04, 0x10065B10), 0x10065AF0, (0x10065AE0, 0x10065ADC, 0x10065AD8), 0x10065AC0
TRACK, TRACK_LAST = 0x10072730, 0x1007272C       # the map; Harry's last tile v, h
LOOP_DONE = 0x10068560                              # GameLoop ends (CheckGameEvents: Harry's death over)
INITS = ("InitPlayerSprite", "InitHeldWeaponSprite", "InitBonusSprite", "InitItemSprite", "InitExitSprite", "InitDoorSprite",
         "InitCoolerSprite", "InitPlantSprite", "InitMessageSprite", "InitColliderSprite", "InitDartEnemySprite",
         "InitRampSprite", "InitEnemyShotSprite", "InitPlayerShotSprite", "InitEffectSprite", "InitTileHotRects",
         "InitPlayerKickRects", "InitBarrelSprite", "InitChairSprite", "InitStairsSprite", "InitButtonSprite",
         "InitSlidingDoorSprite", "InitCoWorkerSprite", "InitMailGuySprite", "InitRobotSprite", "InitFirePipeSprite",
         "InitGrateSprite", "InitGhostSprite", "InitTurretSprite", "InitReversorSprite", "InitBoulderSprite",
         "InitScientistSprite", "InitBlobSprite", "InitBossSprite")
BINDINGS = [0x7B, 0x7C, 0x7D, 0x7E, 0x31, 0x3B, 0x3A]
KEYS = {"l": 0x7B, "r": 0x7C, "s": 0x7D, "i": 0x7E, "k": 0x31, "e": 0x3B, "u": 0x3A, "1": 0x12, "2": 0x13, "3": 0x14,
        "4": 0x15, "W": 0x0D, "A": 0x00, "V": 0x09, "Y": 0x10, "G": 0x05, "R": 0x0F, "P": 0x23, "H": 0x04, "Z": 0x06,
        "B": 0x0B, "E": 0x0E, "N": 0x2D}
PROP_FACES = {0x10065A60: 6034, 0x10065A5C: 6035, 0x10065A58: 6036, 0x10065A54: 6037,
              0x10065A40: 6051, 0x10065A3C: 6052, 0x10065A38: 6053, 0x10065A34: 6054,
              0x10065A0C: 6041, 0x10065A04: 6042, 0x10065A00: 6043, 0x100659FC: 6044}     # the props, the fire pipes
CASES = 1500
ITEMS = [11299, 11599, 11799, 11999, 11899, 11499, 11699]
AMMO = {"staple": 0x10065AEC, "badass": 0x10065AE8, "shrapnel": 0x10065AE4}


def iter_list(m, sat):
    a = m.r32(sat + 0x78)
    while a:
        yield a
        a = m.r32(a + 0x2E)


def s16(x):
    x &= 0xFFFF
    return x - 0x10000 if x & 0x8000 else x


def record(st, data):
    """A saved-game record for a run that continues (see the cases)."""
    d = bytearray(0x95D8)
    d[0:4] = (0x0B07CF71).to_bytes(4, "big")
    d[4] = st["cont"]
    slots = [tuple(map(int, x.split(":"))) for x in st["inv"].split(",")] if st["inv"] != "-" else []
    slots += [(-1, 0)] * (12 - len(slots))
    struct.pack_into(">hi3h", d, 6, st["level"], st["score"], st["comfort"], st["surface"], st["favor"])
    for i, (oid, cnt) in enumerate(slots):
        struct.pack_into(">hh", d, 0x12 + 4 * i, oid, cnt)
    struct.pack_into(">6h", d, 0x42, st["h"], st["v"], st["dir"], st["staple"], st["badass"], st["shrapnel"])
    if st["seen"] != "-":
        for spot in st["seen"].split(","):
            th, tv = map(int, spot.split(":"))
            for x in range(max(th - 7, 0), min(th + 7, 128)):
                for y in range(max(tv - 7, 0), min(tv + 7, 128)):
                    d[0x4E + x * 128 + y] = 1
    gone = set(map(int, st["gone"].split(","))) if st["gone"] != "-" else set()
    for k in range(256):
        e = bytearray(data[0x10000 + k * 16:0x10010 + k * 16])
        if k in gone:
            e[0] = 0
        v, h = struct.unpack(">hh", e[12:16])
        struct.pack_into(">hh", e, 12, v - 6, h - 6)
        d[0x4058 + k * 16:0x4068 + k * 16] = e
    d[0x5058:0x5518] = data[0x11000:0x114C0]
    d[0x5518:0x5522] = b"\x09Episode 1"
    return bytes(d)


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
    # InitFaces: the props' faces (with their HrSp header: the size the
    # drawing goes by), drawn by UserTintableSpriteDrawProc
    user_tintable = {v: k for k, v in tvectors(f"{work}/ppc").items()}["UserTintableSpriteDrawProc"]
    for slot, hrsp in PROP_FACES.items():
        rec = m.alloc(0x1C)
        m.write(rec, open(f"{work}/res/Harry_Graphics/HrSp_{hrsp}.bin", "rb").read()[:16])
        faces.ids[rec] = hrsp
        m.w32(m.glob(slot), rec)
        m.w32(rec + 0x18, user_tintable)
    # the face loaders give every face a draw proc, BlitEncFaceProc unless
    # the game sets another; the stubbed loaders leave it at 0
    plain = m.alloc(8)
    m.w32(plain, m.addr("BlitEncFaceProc"))
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
              "LoadLevelSounds", 0x1005F7EC, "IsPressed", "AddRectToUpdate",
              0x10024DE8, "RedrawGrid", "RedrawFGTiles", "GetScalePort", "CopyToScalePort", "CopyScalePortBack",
              "FadeMusic", "DebugHook", "CopyStatRect",
              "PlotEncFace", "DrawNumItems", "FindInvItemFace", "UpdateInventory", 0x1005F738, "DrawBlackLines"):
        m.stub_routine(f, 0)
    m.stub_routine(0x10056DE8, 0)                   # nothing is playing
    # BlitEncFace(face, port, ..., mode, arg): the draw procs' tints, but not
    # the fans' grilles and the dart enemies' darts (other faces than the
    # one the draw proc was called with through __ptr_glue)
    blits, proc_face = [], [0]

    def glue(uc, _a, _s, _u):
        proc_face[0] = m.get(3)
    m.uc.hook_add(UC_HOOK_CODE, glue, None, 0x1006536C, 0x1006536C)

    def blit(mm):
        if mm.get(3) == proc_face[0]:
            blits.append("%s:%d:%d;" % (faces.ids.get(mm.get(3), "?"), s16(mm.get(9) & 0xFFFF), s16(mm.get(10) & 0xFFFF)))
        return 0
    m.stub_routine("BlitEncFace", blit)

    def level_start():
        inv_copy = m.glob(START_INV)
        return ":".join([str(s16(m.r16(m.glob(a)))) for a in START_SHORTS] +
                        [str(struct.unpack(">i", m.read(m.glob(START_SCORE), 4))[0])] +
                        [str(s16(m.r16(m.glob(a)))) for a in START_AMMO] +
                        [f"{s16(m.r16(inv_copy + 4 * i))}/{s16(m.r16(inv_copy + 4 * i + 2))}" for i in range(12)])

    def track():
        """The track map: how many tiles are seen and the sum of their
        indices + 1; Harry's last tile."""
        cells = m.read(TRACK, 128 * 128)
        seen = [i for i, c in enumerate(cells) if c]
        return "%d:%d:%d:%d" % (len(seen), sum(i + 1 for i in seen), s16(m.r16(TRACK_LAST + 2)), s16(m.r16(TRACK_LAST)))

    def saved():
        """The saved-game record: copier flag, level, score and the sum of
        all bytes times their index + 1."""
        d = m.read(m.r32(m.r32(m.glob(0x10065B60))), 0x95D8)
        return "%d:%d:%d:%d" % (d[4], struct.unpack(">h", d[6:8])[0], struct.unpack(">i", d[8:12])[0],
                                sum((i + 1) * b for i, b in enumerate(d)) & 0xFFFFFFFF)

    def ripple():
        """The ripple: phase, the level's length, width and speed, the sum
        of the sine table's entries times their index + 1."""
        lvl = m.r32(m.r32(m.glob(0x10065B64)))
        n, wd, sp = struct.unpack(">3h", m.read(lvl + 0x12088, 6))
        tab = struct.unpack(">%dh" % max(n, 0), m.read(m.glob(0x10065D04), 2 * max(n, 0)))
        return "%d:%d:%d:%d:%d" % (struct.unpack(">i", m.read(m.glob(0x1006592C), 4))[0], n, wd, sp,
                                   sum((i + 1) * x for i, x in enumerate(tab)))

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
    # CopierSave runs: its dialog answers from the run's list (1 save, 2
    # cancel), SaveSG (the file) succeeds; DoSaveGame runs too (autosave)
    copier_answers = []

    def modal_dialog():
        item = copier_answers.pop(0) if copier_answers else 2
        log.append("save%d" % (item == 1))
        m.w16(m.get(4), item)
    for n in ("GetNewDialog", "SetPort", "BeginUpdate", "GetDialogItem", "InsetRect", "PenSize", "FrameRoundRect",
              "EndUpdate", "DrawDialog", "ShowWindow", "SelectWindow", "DisposeDialog", "FlushEvents", "GetDateTime"):
        m.stub(n, m.tb_nop)
    m.stub("ModalDialog", modal_dialog)

    def sine():
        """MathLib sin: f1 in, f1 out."""
        x = struct.unpack(">d", m.uc.reg_read(UC_PPC_REG_FPR1).to_bytes(8, "big"))[0]
        m.uc.reg_write(UC_PPC_REG_FPR1, int.from_bytes(struct.pack(">d", math.sin(x)), "big"))
    m.stub("sin", sine)
    for f in ("SaveSG", "RestoreWindowBackground", "ShowCursorSafe", "HideCursorSafe", 0x1005FBA8):
        m.stub_routine(f, 1 if f == "SaveSG" else 0)
    m.w32(m.glob(0x10065B60), m.new_handle(bytes(0x95D8)))
    # ContinueGame: OpenSG brings the run's record, GameLoop only says
    # where it would start (the recorder does its level start)
    cont_record, loop_args = [b""], []

    def open_sg(mm):
        mm.write(mm.r32(mm.r32(mm.glob(0x10065B60))), cont_record[0])
        return 1
    m.stub_routine("OpenSG", open_sg)
    m.stub_routine("GameLoop", lambda mm: loop_args.append(tuple(s16(mm.get(i)) for i in (3, 4, 5, 6))) or 0)
    m.stub_routine("OpenDefaultWorld", 1)              # the world is loaded
    for f in ("DrawPICTToBackScreen", "ResetCursor", "ShowMenuBar", "HideMenuBar", "DrawMainMenu", "UpdateMenus",
              "AskToContinue"):
        m.stub_routine(f, 0)
    m.write(m.glob(0x1006598C), b"\x09Episode 1")     # the world file's name
    # UpLevel runs: the pictures, dialogs, saving and music stubbed, the
    # unregistered check answering "registered", the next level's data
    # put where OpenDefaultWorldLevel would (SetupLevelSprites loads the
    # rest)
    ul = m.addr("UpLevel")
    m.uc.hook_add(UC_HOOK_CODE, lambda uc, a, s_, _: log.append("uplevel"), begin=ul, end=ul)
    m.stub_routine("Win", lambda mm: log.append("won") or 0)
    for f in ("EndLevelEffect", "RestorePreExistingWindowBackground", "ScrollPlot", "LevelLimit",
              "IntroLevel", "SetMusic", "UpdateText", "RedrawGameArea"):
        m.stub_routine(f, 0)
    m.stub_routine(0x10057CB0, lambda mm: mm.write(mm.get(3), b"\1") or 0)

    def open_level(mm):
        data = levels[mm.get(3) & 0xFFFF][0]
        h = mm.alloc(4)
        p = mm.alloc(len(data), clear=False)
        mm.write(p, data)
        mm.w32(h, p)
        mm.w32(mm.glob(0x10065B64), h)
        return 1
    m.stub_routine("OpenDefaultWorldLevel", open_level)
    for n in ("UnionRect", "ForeColor", "BackColor", "PaintRect", "RGBForeColor", "TextFont", "TextSize", "TextFace",
              "MoveTo", "DrawString", "NumToString", "LineTo", "FrameRect", "EraseRect", "InvertRect"):
        m.stub(n, m.tb_nop)
    # UpdateMeters' drawing: a value strip is CopyBits from PICT 150's row
    # 11 / 22 / 33 (length = the source's right edge), the rounded end
    # FUN_10062a0c(face, port, point, ...) at v 387 / 417 / 447
    meters_drawn = [[0, 0] for _ in range(3)]

    def copy_bits():
        top, _left, _bottom, right = struct.unpack(">4h", m.read(m.get(5), 8))
        if top in (11, 22, 33):
            meters_drawn[top // 11 - 1][:] = [right, 0]
    m.stub("CopyBits", copy_bits)

    def meter_end(mm):
        v, h = s16(mm.get(5) >> 16), s16(mm.get(5) & 0xFFFF)
        if v in (387, 417, 447):
            meters_drawn[(v - 387) // 30][1] = 1
            assert h - 473 == meters_drawn[(v - 387) // 30][0]
        return 0
    m.stub_routine(0x10062A0C, meter_end)
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
        entries = [struct.unpack(">hh", data[0x1000C + i * 16:0x10010 + i * 16]) + struct.unpack(">h", data[0x10002 + i * 16:0x10004 + i * 16])
                   for i in range(256) if data[0x10000 + i * 16]]
        levels[n] = (data, start, entries)
    load_words(m, work)
    # the world (Hwld 0): UpLevel reads its number of levels (+0x1c6)
    m.w32(m.glob(0x10065B68), m.new_handle(open(f"{work}/res/Episode_1/Hwld_0.bin", "rb").read()))
    # a new game (the level-start state: 72 / 72 / 72, nothing else)
    m.stub_routine("InitDynamicSounds", 0)
    m.call("InitGameGlobals")
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
              "comfort": rnd.choice([144, 100, 20, 144, 100, 20, 0]), "favor": rnd.choice([144, 60, 5]), "frames": rnd.choice([rnd.randint(1, 60), rnd.randint(1, 300)]), "h": 0, "v": 0, "dir": 0,
              "inv": "-", "curslot": -1, "staple": 0, "badass": 0, "shrapnel": 0,
              "balloons": "".join(rnd.choice("0001") for _ in range(16)), "copier": "".join(rnd.choice("12") for _ in range(4)),
              "autosave": rnd.choice([0, 1])}
        if rnd.random() < 0.6:
            items = rnd.sample(ITEMS, rnd.randint(1, 4))
            st["inv"] = ",".join(f"{i}:{rnd.randint(1, 3)}" for i in items)
            st["curslot"] = rnd.randint(0, len(items) - 1)
            for k in AMMO:
                st[k] = rnd.choice([0, 3, 50, 100, 150, -3])
        # Harry at the level's start, next to an exit or next to one of its sprites
        exits = [e for e in entries if 9990 <= e[2] <= 9999]
        if exits and rnd.random() < 0.1:
            e = rnd.choice(exits)
            st["h"], st["v"] = e[1] - 6 + rnd.randint(-40, 20), e[0] - 6 + rnd.randint(-40, 20)
            st["dir"] = rnd.randint(0, 35)
        elif rnd.random() < 0.3:
            st["h"], st["v"], st["dir"] = start_h - 32, start_v - 32, start_dir
        else:
            e = rnd.choice(entries)
            st["h"], st["v"] = e[1] - 6 + rnd.randint(-70, 40), e[0] - 6 + rnd.randint(-70, 40)
            st["dir"] = rnd.randint(0, 35)
        # or from a saved game: a copier save (where Harry is) or one at the
        # level's start, with other meters, score, track map and sprite list
        st["cont"] = -1
        if rnd.random() < 0.15:
            present = [i for i in range(256) if data[0x10000 + i * 16]]
            st["cont"] = rnd.choice([0, 1])
            st["surface"], st["score"] = rnd.randint(1, 144), rnd.randint(0, 200000)
            st["gone"] = ",".join(map(str, rnd.sample(present, min(len(present), rnd.randint(0, 6))))) or "-"
            st["seen"] = ",".join(f"{rnd.randint(0, 127)}:{rnd.randint(0, 127)}" for _ in range(rnd.randint(0, 5))) or "-"
        held, keys = "", []
        for _f in range(st["frames"]):
            if rnd.random() < 0.15:
                pool = "llrrsssssiikk" + ("uuuue1234" if st["inv"] != "-" else "")
                held = "".join(sorted(set(rnd.sample(pool, rnd.choice([0, 1, 1, 1, 2])))))
            keys.append(held or "-")
        # W-A-V-Y-G-R-A-V-Y typed (once or twice): the ripple on / off
        if rnd.random() < 0.06:
            f0 = rnd.randint(0, st["frames"])
            for _ in range(rnd.choice([1, 1, 2])):
                for c in "WAVYGRAVY-":
                    if f0 < len(keys):
                        keys[f0] = c
                    f0 += 1
                f0 += rnd.randint(0, 40)
        # P-A-R-G-H-Z (a random message) or B-E-N (Ben's) typed: a balloon
        if rnd.random() < 0.06:
            f0 = rnd.randint(0, st["frames"])
            for c in rnd.choice(["PARGHZ-", "BEN-"]):
                if f0 < len(keys):
                    keys[f0] = c
                f0 += 1
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
        copier_answers[:] = [int(c) for c in st["copier"]]
        m.write(m.r32(m.r32(m.glob(0x10065B60))), bytes(0x95D8))   # the record, cleared
        m.write(prefs + 1, bytes([st["autosave"]]))
        # the level start of GameLoop
        h, v, start_dir_ = st["h"], st["v"], st["dir"]
        if st["cont"] >= 0:
            cont_record[0] = record(st, data)
            loop_args.clear()
            m.call("ContinueGame", 0)
            h, v, start_dir_, _intro = loop_args[0]
        m.call("SetupLevelSprites")
        m.call("BuildSineTable")                        # GameLoop's IntroLevel
        for d in meters_drawn:
            d[:] = [0, 0]
        p = m.call("SATNewSprite", 0, h, v, 0x10066FB4)
        m.w32(m.glob(PLAYER_SPRITE), p)
        m.w16(p + 0x66, start_dir_)
        m.w16(m.glob(PLAYER_H), h - 32)
        m.w16(m.glob(PLAYER_V), v - 32)
        m.write(target, struct.pack(">hh", v - 208, h - 208))
        m.write(view, struct.pack(">hh", v - 208, h - 208))
        m.call("SATNewSprite", 0, h, v, 0x1006724C)
        m.call("UpdateMeters", 0)                       # GameLoop: all meters drawn
        m.call("FindUpperLeftCorner")
        scroll()
        m.write(LOOP_DONE, b"\0")                    # GameLoop clears it before the loop
        harry = []
        crc, nblits = 0, 0
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
            blits.clear()
            if debug_seed and os.environ.get("PROPS"):
                print("  props", [struct.unpack(">BxhhI", m.read(0x100723AC + 12 * k, 10)) for k in range(3)],
                      "view", struct.unpack(">hh", m.read(view, 4)), "rect", struct.unpack(">4h", m.read(0x10067B6C, 8)))
            draw()
            nblits += len(blits)
            crc = zlib.crc32(("".join(blits) + "|").encode(), crc)
            if debug_seed == st["seed"] and os.environ.get("BLITS"):
                print("  blits", "".join(blits))
            # GameLoop: the ripple's phase grows by the level's speed
            lvl = m.r32(m.r32(m.glob(0x10065B64)))
            ph = m.glob(0x1006592C)
            phase = struct.unpack(">i", m.read(ph, 4))[0] + s16(m.r16(lvl + 0x1208C))
            m.write(ph, struct.pack(">i", (phase + 2**31) % 2**32 - 2**31))
            m.call("UpdateMeters", 1)
            m.call("UpdateAmmo", 1)                     # clamps the ammunition to 0-100
            ups = log.count("uplevel")
            m.call("CheckGameEvents")
            if log.count("uplevel") > ups and "won" not in log:
                m.call("BuildSineTable")                # UpLevel's IntroLevel (stubbed) builds it
            m.call("UpdateTrackMap")
            p = m.r32(m.glob(PLAYER_SPRITE))                # UpLevel makes him anew
            harry.append("%d:%d" % (m.r16(p + 0x5A), m.r16(p + 0x5C)))
            if debug_seed == st["seed"]:
                print(f, keys[f], dump(m, p, faces, tv), "comfort", m.r16(COMFORT), "slot", m.r16(CUR_SLOT), "rand", m.rand_seed,
                      "log", log[-3:], "list", ",".join(str(m.r16(a)) for a in iter_list(m, sat)))
                if os.environ.get("ADDR"):
                    print("   @", dump(m, int(os.environ["ADDR"], 16), faces, tv))
                for a in iter_list(m, sat):
                    if m.r16(a) == int(os.environ.get("KIND", "-1")):
                        print("   ", dump(m, a, faces, tv))
            if m.read(LOOP_DONE, 1)[0]:
                if "won" not in log:
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
             "curslot": m.r16(CUR_SLOT), "ammo": ":".join(str(m.r16(m.glob(a))) for a in AMMO.values()),
             "score": struct.unpack(">i", m.read(m.glob(SCORE), 4))[0], "start": level_start(), "track": track(), "save": saved(), "ripple": ripple(), "blits": "%d:%d" % (nblits, crc),
             "meterdraw": ",".join("%d:%d" % tuple(d) for d in meters_drawn), "harry": ",".join(harry),
             "list": ";".join(f"{ids[a]}/{dump(m, a, faces, tv).replace(' ', ':')}" for a in lst),
             "free": ",".join(str(ids[a]) for a in free) or "-"}
        out.write(" ".join(f"{k}={v}" for k, v in st.items()) + " -> " + " ".join(f"{k}={v}" for k, v in o.items()) + "\n")
    out.close()
    print(f"game: {CASES} cases")


if __name__ == "__main__":
    main()
