# Harry the Handsome Executive - notes on the original

How the original works: files, formats, code. Every finding goes here,
also the small ones - this is the specification of the port.

## The original

- **Harry the Handsome Executive 1.0.1** (released 1997-11-07), Ambrosia
  Software; author Ben Spees (Easter egg "Ben"), music Eric Speier.
  Shareware ($ registration via `Register Harry`, 30 day licence text).
- Version history (from `Harry 1.0.1 Notes.text`): 1.0.0 released
  1997-09-20; 1.0.1 fixed music crashes, "things happening twice" in
  Reduce Frame Rate mode, boulders, the "object of kind 0" dialog, a
  missing door in level 13; lowered the requirement to System 7.0.
  1.0.2 was announced (1998-12-09) but never released.
- Source: `harry.bin` = MacBinary II of **Harry Installer** (type `APPL`,
  creator `VIS3`, 1999-01-07): an **Installer VISE** (MindVision) archive.
  The data fork starts with `SVCT` - VISE's proprietary compressed archive;
  `unar` does not read it. Installed in Basilisk II (System 7.5, Easy
  Install) onto an empty HFS disk; files taken out with `hcopy -m`.
- Also available: `Harry_Archive_2026.sit` (John Parr's fan archive v1.3,
  Jan 2026): the same `harry.bin`, the 1.0.0 installer
  (`Outdated files/Harry Installer.sit`, a different, older installer
  format, data fork starts `?\xe0\xee[` ... `EXTENSIONS` `Harry
  Instruments`), Harry Level Editor b4, Harry Cheater, Harry REF, Saved
  Game Editor, InvertedHARRY (modified `Episode 1` + `Harry Graphics`),
  ~27 custom worlds, the soundtrack as MP3 (fan made, 2005), screenshots,
  an Easter egg list (in its README).

## Installed files (1.0.1)

| File | Type/Creator | Contents |
|---|---|---|
| `Harry 1.0.1` | APPL/Hary | the game, **FAT**: 68k CODE + PowerPC PEF in the data fork |
| `Episode 1` | Hwld/Hary | the world: 15 levels + level −1, world info, tile sets, story pictures |
| `Harry Graphics` | Hrdt/Hary | 929 sprite faces (`HrSp`), cicn, 4 PICT |
| `Harry Music` | Hrdt/Hary | 17 `Tune` (QuickTime Music Architecture) |
| `Harry Sounds` | Hrdt/Hary | 100 `snd ` |
| `Harry Titles` | Hrdt/Hary | title/menu/game screens, instructions, messages |
| `Harry Instruments` | thng/dvbM | QuickTime music component with 35 `ssai` sampled instruments (2.2 MB); installed into System Folder:Extensions |
| `Register Harry` | APPL/Areg | Ambrosia registration utility |
| `Extras ƒ:Harry Editor 1.0.0b3` | APPL | level editor |
| `Screenshots ƒ` | PICT files | levels 10, 12, 13 |

A custom world is a file of type `Hwld` with the same resources as
`Episode 1` (the editor FAQ: tile set PICTs and imported MIDI music
live in the world file).

## The application

Resources (141): `CODE` 0–7, `DATA` 0, `cfrg` 0 (PowerPC fragment in the
data fork, 437 407 bytes, `Joy!peffpwpc`, imports InterfaceLib, MathLib,
SoundLib, QuickTimeLib), blitters as code resources (`MBlt` 8 / `RBlt` 8
68k mask/rect blitter, `PMBl` / `PRBl` 8 the PowerPC ones), dialogs
(DLOG/DITL/dctb: 030 warning, Copying Machine, Level Limit, Generic
Message, Game Over, Warp, scrolling credits, Memo, Dialogue, New Game,
Save Game, Preferences with Game/Keyboard/Sound panels), `MENU` 128–130,
600/601, `STR#` (warnings, fatal errors, messages, names and a buzzword
generator: Templates/Nouns/Verbs/Descriptors/Adjectives/Objects 6900–6905),
`Tune` 6900 (named "dummy"), `WDEx` 128 (Infinity Windoid 3.0 WDEF),
registration resources (TEXT/STR#/ALRT 900–902).

### 68k code: CodeWarrior, far model, with MacsBug names

| Seg | Name | Size | Routines |
|---|---|---|---|
| 1 | Main | 52 362 | 100: init, level loading, `GameLoop`, menus, events, `main` |
| 2 | Sources | 62 770 | 135: blitters, gamma fades, sound (3D/stereo), physics helpers, tiles, files, buzzwords, music (QuickTime), prefs, scrolling |
| 3 | Sprite Source | 167 311 | 211: player and every sprite kind (Init/SETUP/HANDLE/HIT per kind) |
| 4 | Mac Libraries | 3 478 | runtime |
| 5 | ANSI Libraries | 418 | runtime |
| 6 | Util Source | 21 604 | 65 |
| 7 | Libs | 37 231 | 115: **SAT** (Sprite Animation Toolkit, Ingemar Ragnemalm) - `SATNEWSPRITE`, `SATREDRAW`, sound queue... |

- Every routine carries its **MacsBug name** after its `rts`
  (`re/tools/macsbug_syms.py` → `re/work/app/symbols.txt`, 626 routines
  with start/end offsets). Names in capitals are Pascal-convention
  routines (CodeWarrior upper-cases `pascal` functions).
- The PowerPC fragment carries the same names (`.InitSAT`, `.GameLoop`,
  ...) - a second, independent view of the same source.
- **Not THINK C:** CODE 0 has a single jump table entry (above A5
  0x18670, below A5 0x1940, table size 8); CODE 1's header is `0000 0001`
  and its startup loads `CODE` resources itself (`'CODE'` constant at
  CODE 1 + 8). Code uses **absolute long addresses** (e.g. `move.w #6,
  $00013c20`), relocated at load time - CodeWarrior's far code / far data
  model. `DATA` 0 (12 841 bytes) is CodeWarrior's compressed A5 data
  (contains C strings: `PICT`, `RBlt`, `MBlt`, `cicn`, "Out of memory! Try
  incr..."). The skill's `thinkc_globals.py` and jump table resolution do
  not apply; the relocation format must be read from CODE 1's startup.
- Landmarks: `GameLoop` CODE 1 0x73f4–0x792e, `main` CODE 1
  0xc910–0xcc7e, `CheckGameLoopKeys` 0x6476, `HandleSprites` 0x6938,
  `GenerateSprite` 0x3304, `LoadLevelPhysics` 0x3c5e, `SetupLevelSprites`
  0x40d8.

### PowerPC code: the reference

Decision (2026-10-22): the **PPC build is read as the reference**; the 68k
build serves for cross-checks and (later, with a loader) for differential
tests against Basilisk II.

- `Harry.pef` (data fork): 3 sections - code 414 604 bytes, data 35 168
  (packed 17 167), loader 5 504. Ghidra's PEF loader with **processor
  `PowerPC:BE:32:default`** (auto-detection picks a 64-bit VLE variant -
  wrong). Code loads at 0x10000000. Ghidra's `PefDebugAnalyzer` aborts
  (read at 0x1006538c), so it creates only the functions reached by
  direct calls.
- **Traceback tables** (AIX style, after every routine: zero word, fixed
  part, `tb_offset` = routine length, name) give exact starts and names:
  `re/tools/pef_traceback.py` → 505 routines. `re/ghidra/ApplyNames.java`
  creates and names them (142 were missing, among them every routine only
  called through a pointer, e.g. `SetupPlayerSprite`,
  `HandlePlayerSprite`).
- All game routines are named. Unnamed (no traceback): the **SAT** library
  (68k CODE 7 names: `SATNEWSPRITE`, `SATRUN`, ...; SAT's source is
  public), and the tile scrolling routines of 68k CODE 6 (`WRAPCOPY`,
  `TILESCROLLSYNCH`, `SCROLLTO`, `REDRAWWHOLEGRID`, ...), probably a
  separately compiled module. E.g. `FUN_10024750` (called by `GameLoop`
  with the view position) is one of them.
- Toolbox calls appear as `.glue::TickCount` etc. (imports from
  InterfaceLib, QuickTimeLib, SoundLib, MathLib). Globals are reached via
  TOC entries (`_DAT_10065b64` = pointer to the level handle, ...); name
  them as they become clear.
- **Reproduce: `re/ghidra/run.sh`** (import, all scripts below, decompile
  → `re/work/ppc/harry.c`, 1043 functions). The scripts, in order:
  - `ApplyNames.java`: functions and names from `traceback.txt`.
  - `NopTOCRestore.java`: CodeWarrior reloads r2 after every call
    through glue (`lwz r2,0x14(r1)`, 2522 times); Ghidra then loses the
    TOC and shows `unaff_r2 + -0x78cc`. The script replaces these loads by
    nops *in the Ghidra database only*.
  - `SetTOC.java`: r2 = TOC = **0x1006d390** over the whole code block.
  - `NameTOC.java`: TOC slots pointing to a TVector {code, TOC} become
    `TV_<function>` (154), so dispatch tables read `TV_SetupRobotSprite`.
  - `ApplyLabels.java` + `re/ghidra/globals.txt`: our names for globals
    (`gComfort`, `gImpulse`, ...); `@slot` lines name the target of a TOC
    slot and the slot itself `p_<name>`.
  - `ApplyStruct.java` + `re/ghidra/sprite.txt`: the `Sprite` record
    (0xa2 bytes) as type of the first parameter of 153 sprite routines
    (`sp->dir`, `sp->speedH`, ...).
  - `DumpData.java`: the unpacked data section → `data.bin` (constants).
- Decompiler artefact: in `switch` cases that store through a pointer
  register set before the jump table (`DoScoot`, `DoKick`, `DoInch`),
  stores appear as `unique0x…` / `register0x…`. They all go to the same
  global (there: `gImpulse`, r31 = r2 − 0x48fa); check with
  `re/tools/ppcdis.py`.
- The hand-written blitters (`BlitEncFace…`, `DoubleBlit…`) use r2 as a
  normal register; their `unaff_r2` is genuine.

### Main loop and timing (`GameLoop`)

- Level start: `SetupLevelSprites`, player created via SAT (`FUN_10060f20`
  = SATNewSprite: kind, h, v, setup TVector), story picture scrolled if
  the level's info #6 is set (`ScrollPlot`), `IntroLevel`, music from
  level offset 0x120fa.
- One pass: `HandleIdleSprites`, `HandleSprites` (SAT run: each sprite's
  task), view position (`FindUpperLeftCorner`), `CheckTileCollisions`,
  erase / draw behind tiles / foreground tiles / draw above tiles, ripple
  counter += level 0x1208c, scroll + dirty rect redraw, `UpdateText`,
  `UpdateMeters`, `UpdateAmmo`, `CheckGameEvents`, `CheckGameLoopKeys`,
  `UpdateDynamicSounds`, `UpdateTrackMap`, `SystemTask` if prefs[5].
- **Timing** (confirmed in SheepShaver: holding ← for 300 ms turns Harry
  by 9 steps = 90°): the pass waits until `TickCount() − 2 ≥ start` → at most
  **30 passes per second** (2 ticks of 1/60 s). Slower Macs simply ran
  slower (no catch-up). "Reduce Frame Rate" (prefs[0]): logic every pass,
  drawing every second pass, waits 4 ticks after a drawn pass → logic
  still 30 Hz, picture 15 Hz.
- Ends when the "level done" flag is set; then `EndLevelEffect` if comfort
  or corporate favor dropped below 1.

### The view (`FindUpperLeftCorner`)

- The view is the top-left corner (v, h) of the 448 × 448 game area
  (`_DAT_100659bc`), set once per frame after the sprites moved. The
  scroll preference (prefs +8): **0** still; **1** follow - a target
  (`_DAT_10065984`) eases toward Harry's top-left − 208 by 1/16 of the
  distance (at least 1 px) and the view is the target; **2** lead - the
  same target, the view at target − 2 × (target − (Harry − 208)), i.e.
  ahead of him; **3 / 4** pages - when Harry's centre is more than 144 px
  from the view's middle (or 145 px across), the view scrolls in that
  direction (straight when the other axis is under 73 px, else diagonal)
  176 px in 12 steps (8, 16 × 10, 8) or 160 px in 6 steps (16, 32 × 4,
  16).
- Then, in every mode: prefs +2 rounds v down to even; a shake counter
  (`_DAT_10065ad0`) moves the view by ±counter / 2 (alternating, not
  while paging) and counts down; Harry's centre is kept within 196 px of
  the (old) middle; the corner is clamped to 0…3648.
- `harry_core::view` with `re/tools/emu_view.py`: 6000 walks of up to 40
  frames in all five modes, all matching; the surviving mutations are
  equivalent (shift vs. division of a positive number, edges that move
  by 0).

### Meters (`UpdateMeters`, panel x 473–620)

| Meter | value | shown | y |
|---|---|---|---|
| COMFORT | `gComfort` 0x1006856a | `gComfortShown` | 386 |
| SURFACE RESISTANCE | `gSurfaceResistance` | `gSurfaceShown` | 416 (drawn inverted: 144 − x) |
| CORPORATE FAVOR | `gCorporateFavor` | `gFavorShown` | 446 |

Range 0–144 (0x90, clamped). The shown value moves 1 per frame toward the
value. Death (`HandlePlayerSprite`: task becomes
`HandlePlayerSpriteDeath`) is tested on the **shown** values
(`gComfortShown < 1 || gFavorShown < 1`), so it happens up to 144 frames
after the real value hit 0. Surface resistance is only a display of the
floor friction: 5 · f − 1150 (min 8), f = 245 → 75.

### Player movement (CODE 3 / PPC `HandlePlayerSprite` and helpers)

Harry sits in a swivel chair. Units: position as 24.8 fixed point
(`fixH`, `fixV`), speed in 1/256 pixel per frame (`speedH`, `speedV`),
direction `dir` 0–35 in 10° steps.

- **Keys** (`HandleKeys`, `GetKeys`; key codes in the prefs at +0x10…;
  defaults from the Key Controls panel of the running original):
  +0x10 Rotate Left (← , dir + 1), +0x12 Rotate Right (→, dir − 1), +0x14
  Scoot (↓, `gScootTimer` = 21), +0x16 Inch (↑, `gInchTimer` = 17), +0x18
  Power Kick (Space, `gKickTimer` = 21) - a new move only when no timer
  runs; +0x1a Select Item (Control), +0x1c Use Item (Option); Toggle Map
  (Tab). Fixed: Pause = Caps Lock, Abort Game = Escape. Scoot moves Harry
  in the direction he faces (start: facing up, dir 27).
  dir wraps 36 → 0, −1 → 35 (in `HandlePlayerSprite`).
- **Moves** (`MoveChair`, every frame unless `sp->hurt ≥ 23`): base thrust
  p = physics[0] (50); × 2.5 while caffeine (`gCaffeineTimer > 0`, 270
  frames after a pill; the multiplication is done in double); × (256 −
  gWet) / 256. `DoScoot(p)`, `DoKick(physics[1] = 70)`, `DoInch(p)` set
  `gImpulse` and the pose (`gPoseOffset` 0/36/72/108 = one of 4 sets of
  36 frames, 144 faces from PICT 200 in Harry Graphics); then
  `ApplyModedImpulse(sp, sp->dir, gImpulse)`.
  - Scoot, by `gScootTimer` (set to 21 by the key, decremented in
    `HandlePlayerSprite` before `MoveChair`, so the first frame sees 20;
    read from the jump table at TOC − 0x13b8 - Ghidra's case labels for
    this switch are wrong, always check jump tables with `ppcdis.py`):

    | timer | 20–18 | 17 | 16 | 15 | 14 | 13 | 12 | 11 | 10 | 9 | 8 | 7 | 6 | 5 | 4 | 3 |
    |---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
    | pose | 36 | 36 | 0 | | | 72 | | | 108 | | | | 72 | | | 0 |
    | gImpulse | | p | 2p | 3p | 4p | 4p | 4p | 3p | 2p | p + p/2 | p | p/2 | 0 | 0 | | |

    Empty = unchanged. p/2 is an arithmetic shift. Timer 17 also plays the
    scoot sound by floor kind (and one by `gWet`).
  - Inch, by `gInchTimer` (key sets 17, first frame sees 16; table TOC −
    0x132c):

    | timer | 16 | 15 | 14 | 13 | 12 | 11 | 10 | 9 | 8 | 7–6 | 5–3 |
    |---|---|---|---|---|---|---|---|---|---|---|---|
    | pose | 72 | | 72 | | | 0 | | | 36 | | 0 |
    | gImpulse | −p/2 | −p | −(p + p/2) | −2p | −2p | −(p + p/2) | −p | −p/2 | +2p | (+2p stays) | 0 |

  - Kick, by `gKickTimer` (key sets 21, first frame sees 20; k =
    physics[1] = 70; table TOC − 0x1370). `gKickContact` is the firmness
    of whatever the player touches this frame (reset every frame; wall 256
    in `HitPlayerTileSprite`, water cooler 192/256, barrel / chair / plant
    / robot 128, blob / boss 64):

    | timer | action |
    |---|---|
    | 20 | pose 72, **timer := 16** (cases 19–17 never run) |
    | 15 | pose 72 |
    | 14 | pose 108, `hotRect` := (0, 0, 64, 64) for this frame (the kick's reach) |
    | 13 | contact c ≠ 0: `gKickPower` := c, gImpulse = (10 · k · c) >> 8, sound; no contact: **timer := 11, gImpulse = 50** |
    | 12 | gImpulse = (9 · k · gKickPower) >> 8 |
    | 11 | gImpulse = (7 · k · gKickPower) >> 8 |
    | 10 | pose 72 |
    | 8 | pose 0, gImpulse 0 |
    | 7 | gImpulse 0 |
    | 6 | pose 36 |
    | 4 | pose 0 |

    gImpulse keeps its value between the listed frames, so a kick against
    a wall pushes on frames 13–9 with 10, 9, 7, 7, 7 · k · c / 256, and a
    missed kick still pushes 50 on frames 13, 10 and 9 (explicitly coded,
    so probably intended).
  - All thrust goes in `sp->dir`; negative thrust (inch) backwards.
- **`ApplyModedImpulse(sp, mode, p)`**: mode 0 → speedH −= p, 9 → speedV
  += p, 18 → speedH += p, 27 → speedV −= p; the other 32 directions use
  `speed = (short)(int)((double)speed ± k · (double)p)` with k from a
  table of **hand-typed sines**: 0.174, 0.342, 0.5, 0.643, 0.766, 0.866,
  0.94, 0.985 (data 0x100667f0–0x10066828). Truncation toward zero.
- **Friction** (`HandlePlayerGlide`, by `gFloorKind` under the player):
  1 → f = physics[2] (245), 3 → f = physics[3] (252), 2 → f = 236; f is
  raised toward 256 by `gWet` (f + (256 − f) · drag / 256) and becomes
  (f + 256) / 2 while `sp+0x9c`/glide counter ≥ 88; per axis: |speed| <
  25 → 0 (unless gliding), else speed = (f · speed) >> 8.
- `EnforceMaxSpeed(sp, 2500)`: if |speedH| > 2500, speedV is scaled by
  2500 / |speedH| (integer division) and speedH = ±2500; then the same for
  speedV.
- Then `fixH += speedH`, `fixV += speedV`, `posH = fixH >> 8`, `posV =
  fixV >> 8`. A shake timer adds random −3…+2 to the drawn position.
- `InverseMode(dir)` = dir ± 18 (opposite direction), e.g. for the recoil
  of a shot (`ApplyModedImpulse(shot, InverseMode(dir), 2000)`).

### Tiles and collisions

- **Grid layout: column major.** `GetBGTile(x, y)` / `GetFGTile(x, y)`
  (tile coordinates, clamped to 0–127) read the word at (x · 128 + y) · 2
  of the background (0x0000) / foreground (0x8000) grid.
- **Tile kinds** come from the level's info fields (`LoadTileDefinitions`):
  #16–#39 (0x12094–0x120c3) hold 96 nibbles = the kind of background
  tiles 0–95 (`LookupBGTileKind`; other values pass through);
  #40–#51 (0x120c4–0x120db) hold 24 bytes (high byte first) = the kind
  + 1 of foreground tiles 32–55 (`Unstuff16BitInto8Bits` subtracts 1, so
  0 = −1 = no collision; `LookupFGTileKind`; tiles < 32 and ≥ 56 are their
  own kind, 0xffff = no tile). In Episode 1 all 24 are 0: foreground tiles
  32–55 never collide there (checked against the original in Unicorn for
  levels 1–15, together with all 96 background kinds).
- **Shapes** (`InitTileHotRects`, captured by running it in Unicorn:
  `re/tools/emu_tables.py` → `re/work/tables/tile_rects.txt`; rect =
  top, left, bottom, right): foreground kinds 0–3 half tiles (left, top,
  right, bottom), 4–7 quarters (bottom-left, top-left, top-right,
  bottom-right), 8–50 full, 51/52/53 shrinking squares (4–28, 8–24,
  12–20), 54–63 full; the second rect (redraw) is the same except 51–53
  (full). Background kinds: 3–5 (6, 6, 26, 26), all others (2, 2, 30, 30).
- **`CheckTileCollisions`** (each frame, after `HandleSprites`): for every
  sprite in SAT's list (`next` at +0x2e) with a `hitTileTask`, the 3 × 3
  tiles around the centre of its face, in the order (0,0), (−1,−1),
  (0,−1), (+1,−1), (−1,0), (+1,0), (−1,+1), (0,+1), (+1,+1): if the
  sprite's hot rect meets the tile kind's rect, call
  `hitTileTask(sp, tile position, kind, isForeground)` - foreground
  first, then background (background kind 0 is skipped). Sprites with
  byte +0x8b set also get the foreground redrawn over them.
- **Player against tiles** (`HitPlayerTileSprite`):
  - Foreground, normal: kind < 32 → `WallBounce(…, physics[4] = 150, …)`;
    32–55 → `RectBounceFake` with the background rect table (sic).
    While kicking (`hotRect.left == 0`): pixel-exact mask test
    (`FUN_10054124` with the tile's mask) → `gKickContact` = 256 and the
    position is put back.
  - Background, only while `sp->hurt == 0`: **1** obstacle
    (`BGTileBounce`, kickable), **2** slippery floor (`gFloorKind` = 3,
    friction 252; reset to 1 every frame in `HandlePlayerSprite`), **3**
    comfort −7, **4** comfort −12 (both: "Ouch", 15 frames of immunity,
    only when not kicking), **5** comfort +1 per frame, **6** water
    (`gWet` += 25 up to 200, scoot sound pitched; `gWet` weakens thrust
    and friction, drops by 2 per frame while |speed| > 200).
  - Debug: with the debug flag and key 0x32 (`` ` ``) held, tile
    collisions are off.
- **`WallBounce(sp, kind, tile pos, 32, bounce, hot rect, mask, first)`**
  (foreground kinds 0–31; bounce = physics[4] = 150, i.e. speed × 150/256
  per hit). Reflection: the speed component into the wall becomes
  `−((bounce · s) >> 8)`, the other `(bounce · s) >> 8`, only if the
  sprite moves into the wall; then the sprite is put against the wall and
  `fix = centre << 8`.
  - 0–3: half tiles (left, top, right, bottom). Kind 0 bounces only on the
    first wall contact of the frame (`first`); 1–3 do not check it.
  - 4–7: quarter tiles, tested pixel-exact (`FUN_10054124` with the tile
    mask); the axis is chosen by the penetration depth (< 9 px → push
    out, else treated as a 45° corner: kind 12–15).
  - 8–11: L-shaped (two rects), axis by distance to the corner.
  - 12–15: 45° diagonals: swap speedH/speedV (13, 15) or swap and negate
    (12, 14), then both × −bounce/256.
  - 16–31: curved/mixed pieces: pixel-exact test, then by which half of
    the tile the centre is in: plain H or V reflection or one of the
    diagonals 12–15.
- **`RectBounceFake`** (foreground kinds 32–55): rect vs rect, axis by the
  centre's distance to the rect's centre; reflection written as
  `(−(bounce · s)) >> 8` - **rounds differently** from `WallBounce`
  (−((b · s) >> 8)): for small speeds the result differs by 1. Port 1:1
  and test differentially.
- Comfort changes are collected in `gComfortDelta` and applied once per
  frame in `HandlePlayerSprite` - **but not while caffeine works**
  (`gCaffeineTimer > 0`): caffeine makes Harry immune to comfort loss and
  gain.

### Random numbers

`FUN_10062dc8(n)` (SAT's `SATRand`, 176 callers): `r = Random(); if (r <
0) r = −r; return r % n` (short arithmetic). `Random` is the QuickDraw
Toolbox generator (Park–Miller on `qd.randSeed`; never returns −32768);
`Randomize` seeds it with `GetDateTime(&qd.randSeed)` - every game differs.
A port can use the same generator (seed from the clock, or fixed for
tests); `scripts/machine.py` of the skill already implements `Random`.

### Inventory and items (from `HandleKeys`)

`gInventory`: 12 slots of {short object id, short count}. Item ids are
the editor's: 11299 staple gun (`gStapleAmmo`), 11499 caffeine pill
(sets `gCaffeineTimer` 270), 11599 B.A.D.A.S.S. (`gBadassAmmo`, charged by
holding fire: < 34 frames weak shot −2 ammo, else strong −6), 11699
mystery vial (random of 6: comfort 100, ..., all ammo/comfort 100, all −25
...), 11799 shrapnel gun (`gShrapnelAmmo`, charge levels 8/16/24), 11899
soda can, 11999 power of the swivel (charged; fires a player shot costing
2 comfort, needs comfort ≥ 10).

### Cheats / Easter eggs found in `HandleKeys`

- With the debug flag (`gDebug`, cleared at start, never set in the
  shipped game): F1 comfort 144, F2 staple gun + 100 staples, F12 comfort
  0, `=` an explosion within ±64 px of Harry (not during his blast
  cooldown, +0x96), F13 soda can, F4 item 11397, F3 caffeine pill.
- Shift (0x38) + `=` (0x18) + key code 0x74 held together, with a sound:
  player scale 1.0 ↔ 2.0 (`scale0/1` = 0x200) - the README's "Command
  Shift +" Easter egg. 0x74 is not Command (0x37). Test in SheepShaver
  (Shift + = + Page Up = 0x74, held 400 ms via Quartz key events): no
  visible change - inconclusive (holding toggles the scale every frame the
  sound is not playing); a short press could not be tried yet.
- Key sequences (KeyMap: code c = bit c & 7 of byte c >> 3, confirmed by
  these words), each step must come within a window of 200/200/250
  frames: **P-A-R-G-H-Z** → random balloon of 52 messages (data
  0x10068ba8, 256 bytes each), **B-E-N** → a fixed balloon (0x10068aa8),
  **W-A-V-Y-G-R-A-V-Y** → toggles the ripple effect (level 0x12088 = 100,
  0x1208a = 16, 0x1208c = 0x300 and `BuildSineTable`).
- Keys 1–4 (codes 0x12–0x15) select staple gun / B.A.D.A.S.S. / shrapnel
  gun / power of the swivel if carried.

### Sprite management (SAT list, idle sprites)

- **SAT configuration** (`InitSAT`): `SATConfigure(false, 1, 1, 64)` -
  sort by layer (+0x4c), collision mode 1, search width 64; sprite
  records 0xa2 bytes. SAT's globals at `*0x10065b70`, first sprite at
  +0x78; links next +0x2e, previous +0x32; free list at +0x110.
- **New sprites** (`SATNewSprite` = `FUN_10060cbc(0, 0, kind, h, v,
  setup)`): a record from the free list (fields not cleared by SAT or the
  setup keep old values) or a new one, kind and position set, put at the
  **front** of the list, then the setup routine runs.
- **One frame** (`GameLoop`): `HandleIdleSprites`; `HandleSprites` =
  `SATRun` (each sprite's task in list order, its `hotRect2` = position
  + hot rect right after; one bubble pass forward and one backward by
  layer; the collision pass: each sprite against the following ones while
  their layer is ≤ its own + 64, overlapping `hotRect2` → a's hit task
  with b, then b's with a; sprites without a task get kind 0 and no hit
  task) and then `CheckPlayerCollision` (unless `*_DAT_10065acc`: the
  first sprite with `HandlePlayerSprite` as task against every other
  sprite with a task, `at` + hot rect, `SectRect` → `HitPlayerSprite`);
  the view (`FindUpperLeftCorner`); `CheckTileCollisions`; when drawing,
  `EraseSprites` takes sprites without a task or `frozen` out of the list
  (a frozen level sprite is also marked gone in the level's sprite list
  and its idle slot freed). So "frozen" means dead; with Reduce Frame
  Rate the erase happens only on drawn passes, so dead sprites stay in
  the list (doing nothing, but still colliding) one pass longer.
  `re/tools/emu_sat.py` + `harry_core::sat`: 20 000 cases with dummy
  sprites (tasks that log, kill or move), all matching.
- **Start of a level**: `SetupLevelSprites` (`GenerateSprite` for each
  active record of the level's sprite list, position − 6), then the
  player and then the held weapon with `SATNewSprite` - so the list
  starts weapon, player, level sprites (in reverse).
- **Idle sprites** (255 slots of 0x44 bytes at 0x1006dcf0): most level
  sprites are made through `AddIdleSprite` - created, then at once
  parked: `ActiveToIdleSprite` saves kind, face, dir, layer, at, level
  index, state, timer, hot rect, speed, +0x6e/+0x70, +0x7c, +0x8b, +0x8c,
  scale0/1, air, task, hit task and hit tile task, and kills the sprite
  (task = 0). Each frame `HandleIdleSprites` goes through the slots in
  order: a parked sprite comes back (`IdleToActiveSprite`: `SATNewSprite`
  without setup, the saved fields restored, fix = at << 8, +0x8a = 1,
  +0x82/+0x84 = −1, +0x76 = its slot) when |view.h − at.h + 200| < 348
  and |view.v − at.v + 200| < 348 (view = the upper-left corner, the view
  is 448 × 448); an active one is parked when one of them is > 348 and its
  hot rect at `at` does not touch the view rect. Fields not saved (e.g.
  +0x90, +0x94, +0x96, +0x98…+0x9c, scale2/3) come back with whatever
  the reused record held.
- **SAT records** (`FUN_10060cbc` = new, `FUN_1005cb9c` = kill): killed
  records go onto a free list (LIFO, linked by +0x2e, head at SAT +0x110)
  and are reused first; a reused record gets only its task, hit task and
  SAT's own fields (+0x16, +0x22, +0x26…+0x2d, +0x36, +0x3e, +0x46)
  cleared - so `IdleToActiveSprite`, which runs no setup, leaves +0x6a,
  +0x78, +0x90, +0x94, +0x96, +0x98…+0x9c and scale2/3 as the record's
  last sprite had them. New records come from `NewPtrClear` (record size
  0xa2, set by `InitSAT` via `FUN_1005fd08`). A new sprite without setup
  gets SAT's default task (`_DAT_100666bc`), which `IdleToActiveSprite`
  replaces.
- **`EraseSprites`** (each drawn frame), apart from the drawing: a sprite
  without a task or frozen is killed (frozen with a level index: its level
  entry's byte 0 cleared and its idle slot freed); every other one gets
  +0x8a = 0, +0x86 = face, +0x82 / +0x84 = at (v / h), scale2 / 3 =
  scale0 / 1, +0x94 and +0x96 counted toward 0 while not 0 (negative ones
  further down), and its level entry gets its kind and at + 6.
- `harry_core::world` (`World`: records, list, free list, idle table,
  props) with `re/tools/emu_world.py`: `SetupLevelSprites` on all 15
  levels (cut down to the ported kinds), then frames of
  `HandleIdleSprites` with a moving view, random deaths and
  `EraseSprites` - 3000 cases, list (with record identities), free list,
  idle table, level entries and props all matching. The recorder needs
  `SATConfigure`, the record size and SAT's init flag set by hand.
- **Object ids → sprite kinds** (`GenerateSprite`; "idle" = via
  `AddIdleSprite`, "even" = position rounded to even pixels toward 0):
  301–319 desk (direct); 500–501 cooler, 550 barrel, 560 chair, 601 plant
  (idle, even); 700–799 bonus (idle); 900–949 door (idle, even); 950–999
  sliding door (idle, even; 951 one pixel lower); 1019 / 1029 / 1039 /
  1049 background props (`AddBGProp`), other 1000–1049 dart enemy (idle);
  1100–1299 scientist (idle); 1300–1449 co-worker (idle, h even + 1);
  1600–1699: 1601 / 1611 / 1621 / 1631 mail guy (direct), 1609 / 1619 /
  1629 / 1639 foreground props (`AddFGProp`); 1700–1799 blob; 1800–1849
  robot; 1850–1899 reversor (direct); 1900–1999 turret; 2000–2099 robot;
  2200–2299 boss; 2301–2304 collider (direct); 3000 nothing, 3001 hazard
  (idle, even); 3100, 3101, 9199, 9200, 9300, 9401–9499, 9600, 9800
  collider (idle); 3900–3999 grate (idle, even; v − 1, 3905 / 3915 not); 4200–4299
  fire pipe (4201–4204 with four face sets, idle, even); 5000–5999
  ghost; 6000–6099 boulder (direct); 9900–9909 stairs; 9500–9599 button;
  9700–9799 ramp; 9990–9999 exit; 11001–19999 item; ≥ 21000 message (all
  idle). "Direct" sprites never go idle.
- Sprite kinds in Episode 1 (count over all 15 levels): bonus 443, door
  172, fire pipe 142, robot 121, blob 116, grate 101, cooler 92, message
  90, barrel 87, turret 78, plant 73, mail guy / props 71, scientist 71,
  item 67, collider ≈ 76, ramp 58, button 51, ghost 50, dart enemy / props
  48, sliding door 28, stairs 26, co-worker 21, chair 19, exit 18, boulder
  14, reversor 12, boss 1. Level 1: doors 901 / 913, plants, cooler 500,
  colliders 3100 / 9200 / 9600, background prop 1029, dart enemy 1021,
  exit 9991, messages, ramp 9703, item 11397.

### Sprite kinds in the port

- The port keeps the sprite record as one struct
  (`harry_core::sprite::Sprite`) with the original's fields, since
  `HitPlayerSprite`, the idle sprites and many kinds read other sprites'
  fields. Faces are named by their `HrSp` id: the recorder
  (`re/tools/sprites.py`) stubs `Load1EncFaceFromCicn` and
  `LoadEncFaceSetFromPICT(pict, count, w, h, per row, first / last
  region frame, first HrSp id)` and runs the kind's `Init…Sprite`, so
  every face pointer maps back to its id.
- `re/tools/emu_kinds.py`: per kind, setup on a zeroed record, 0–40
  frames of the task (frozen set at a random frame), the hit task with
  the player / another sprite / a frozen player; the whole record
  dumped. `harry_core::kinds` matches all cases (3 000 per kind); all
  planted mutations caught after widening the frame range and the exit
  destinations.
- **Bonus** (700–799, layer 1, hot rect (−6, −6, 22, 22)): faces 700 /
  701 → 1501 (blinking 1501 / 1502 each frame), 702 → 1503 with hot rect
  to 36, 703 → 1504, 711–713 → 1505–1507 with hot rect (−10, −10, 26,
  26), others none. Hit task empty (Harry's side is in
  `HitPlayerSprite`).
- **Item** (11001–19999, layer 7, hot rect (−6, −6, 20, 20)): face
  3601 + (id − 11199) / 100 (none for 11699 and 12099; ids outside
  11199–12198 read past the table), 11397 → 3652, 11398 → 3651; the
  mystery vial 11699 bubbles through 3661–3668, two frames each. No fix
  position set.
- **Door** (901–933, layer 5, faces 2601–2636: nine per hinge side;
  hot rects 901–903 (34, 15, 37, 46), 911–913 (15, 16, 18, 47), 921–923
  (3, 33, 34, 36), 931–933 (1, 4, 32, 7)): …2 open (dir 16, last face),
  …1 closed, …3 locked. Each frame the face is dir / 2 of its nine; when
  kicked open (dir 1) it counts up to 17 and becomes the open kind at
  once, also in the level's sprite list (so it stays open on reload). A
  closed or locked door stops player and enemy shots (they freeze).
- **Water cooler** (500, 501; layer 8, hot rect (0, 0, 24, 24), face
  2001, mask HrSp 2002): it reacts in its own hit task (SAT's collision
  pass), not in `HitPlayerSprite`. Harry on the floor: kicking - his kick
  rect (`KICK_RECTS`[dir] at his `at`) touching its hot rect - rings it
  (3-D sound 501, kick contact 192); walking - the mask test (his body
  mask at his `at` against the cooler's) moves him out: `at` and fix
  from the moved position, sound 501, speed × −0.6 (double, truncated),
  kick contact 256. Player / enemy shots (frozen) and effects (except
  kind 9610) knock it: `dir` 1 → 2 → 3 with sound 502, 3 → 4 with 500;
  at 4 its task makes an explosion (kind = level info #1, default 5) at
  pos − 20 and marks it gone in the level; at 10 it is removed.
  `re/tools/emu_cooler.py`: 12 000 cases, all matching; 9 of 11
  mutations caught, the other two equivalent (dir > 10 never happens;
  −0.6 · v truncates like −6 · v / 10 for all 16-bit v).
- **Plant** (601; layer 8, hot rect (8, 8, 28, 28), faces 3901–3904,
  bounce mask cicn 599): a hit plant (dir 1) shakes 3902, 3902, 3903,
  3903, 3904, 3904, then back to 3901. Each frame `HandleGlide` (object
  friction: on the floor or at air 24–199 the floor's friction - slippery
  on background kind 2 -, halfway to 1.0 when `slide` > 87; speeds below
  25 stop unless `slide` > 87), `HandleHalfPipe` and `JumpScale` (centre
  + 18, not as the player; at air 999 the unset scale is r30 =
  0x1007dfac of `HandlePlantSprite`), then the straight move. Its tile
  routine bounces off walls (`WallBounce`) and, on the floor, background
  obstacles (`BGTileBounce`) with its own mask, always as first contact.
  `re/tools/emu_plant.py`: 12 000 cases of 1–6 frames each with tile
  collisions on all levels (special floors, wall edges), all matching;
  all 11 mutations caught.
- **Plant hit task** (`HitPlantSprite`, both on the floor): Harry
  kicking it (`KICK_RECTS`[dir] touching its hot rect): it shakes (dir
  1), 3-D sound 1101, flies off away from him (`ApplyModedImpulse` with
  `InverseMode(dir)` and kick thrust × 13 = 910), kick contact 128.
  Walking into it: the mask test (his body at his SAT position against
  cicn 599) moves him out (`at`, fix from it), the plant's fix is reset to
  its `at`, sound 1100 (if not playing), the plant gets 0.6 of his speed,
  he keeps −0.3 (doubles, truncated). Against another plant or a cooler:
  on the same pixel both are jittered apart (`SATRand(0x300)` + 0x100,
  four times), then the mask test (plant mask against cicn 599 for a
  plant, cicn 598 - the barrel's - for a cooler); a hit: fix from the
  moved position, sound 1100, with a plant momentum is traded (other +=
  0.4 · mine, mine += 0.4 · other's new, both × −0.4), with a cooler the
  plant only bounces back (× −0.4). Doors (not open), colliders,
  co-workers: `RectBounce` (centre + 18, the level's bounce). Player and
  enemy shots (except kind 9610) stop on it, 3-D sound 1101, it shakes;
  kind 9450 also pushes it (speed / 4). Not yet: effects
  (`HandleExplodeRadius`), mail guy, barrel, desk, chair, robot, grate,
  ramp, stairs.
- `RectBounce` (PPC 0x1001e4bc): if the hot rects (at `at`) overlap, the
  side is chosen from the centres (h distance against v distance), the
  sprite put just outside, and the speed across reflected (both × bounce
  / 256) only when it moves into the rect.
- `re/tools/emu_planthit.py`: 20 000 cases (Harry kicking / walking,
  plant, cooler, all door kinds, shots), all matching; 15 of 16
  mutations caught - the survivor shifts the jitter by 1/256 px, which the
  mask separation right after almost always overwrites.
- **Message** (21000 and up; layer 9, faces 3801–3805): the mode
  (`f6e`) is the level entry's info #1, read each frame: 0 a memo on the
  floor (hot rect (−8, −8, 32, 32), the face blinking 3801 → 3805 → 3801
  over 30 frames), else an invisible spot (hot rect (−32, −32, 64, 64), no
  face); the kind is set to 21000 each frame. Harry walking (not kicking)
  onto it: `ShowBalloon(sprite, text, mode)` with text = world text
  #(info #0) (`Hwld` + 0x3c9 + n · 256, Pascal strings) or, in mode 3, a
  `GenerateBuzzwords` sentence. Shown → the message is done (frozen,
  except that `ShowBalloon` itself leaves mode 5 alive). Not shown (it
  would not fit on the screen) and prefs +8 > 2 → kept pending (+0x6a,
  a pointer to Harry) and tried every frame.
- **`ShowBalloon`** (frontend part, not ported yet): TextEdit layout in
  font 3 (Geneva) 9 pt, 195 px wide; the balloon (210 px wide, a tail
  under it) goes over the sprite (mode 5: at its left, others: 57 px left
  of it; mode 2 or no sprite: over Harry) and is shown only if it lies
  inside the 448 × 448 view with a 32 px margin - else, with prefs +8 = 3,
  the view is nudged 16 px. Shown: modes 1, 3, 5 play sound 217 pitched
  56000 + `SATRand(16000)`, the game draws it and waits at least 60 ticks
  (Return / Enter skip), then for any key.
- `re/tools/emu_message.py`: 12 000 cases (frames, hit at a random
  frame, ShowBalloon answers recorded), all matching; 9 of 9 mutations
  caught.
- **Exit** (9990–9999, layer 1): 9991–9994 with faces 3201–3204 and hot
  rects beside them (9991 (96, 0, 100, 64), 9992 (−40, 0, −32, 64), 9993
  (0, 40, 64, 100), 9994 (0, −32, 64, 0)), others no face, (0, 0, 32,
  32). Its hit task, with the live player and its timer 0: previous level
  = current, current = the exit's info #0 or current + 1, timer 69 (never
  counted down - once only). The level end itself is noticed elsewhere.

- **Colliders** (`SetupColliderSprite`, layer 5, state 5, `f78` = −1;
  Episode 1: fans 2301 / 2302 / 2303, phones 3100, copiers 9200, magnets
  9300, wall guns 9401 / 9411 / 9421 / 9431, vending machines 9600, one
  gate 9800). Hot rects (top, left, bottom, right): fan (4, 4, 28, 28),
  phone (−18, −18, 34, 34), copier (0, 12, 30, 40), magnet (1, 1, 22,
  22), gun (2, 2, 14, 14), vending (4, 0, 30, 32), gate (0, 8, 64, 56);
  1019 / 1029 / 1039 / 1049 / 2001 only get a hot rect (the props come
  from `AddBGProp`). Faces HrSp 1901–1911, fans 1921–1924. Task by id:
  - **Fan** (2301 up, 2302 down, 2303 left, 2304 right): only while Harry
    is within 448 px on both axes. First time: reach (`f6e`, −69 =
    unset) = info #0 or 125, strength (timer) = info #1 or 256. Sound
    1303 when the fan sound is not playing, the face turns (4 frames),
    then `HandleRepulsions` goes through the SAT list: Harry, shots,
    barrels, blobs, ghosts, plants, chairs, reversors and robots whose
    hot rect centre (`SpriteCenter`) is in front of the fan (at + 16),
    nearer along the wind than across and nearer than the reach, get
    (reach − distance) × strength >> 8 added to their speed.
  - **Phone** 3100: quiet 100–299 frames (timer < 0, start −100 −
    `SATRand(100)`), then rings 1–3 × 90 frames (sound 301 at 270 / 180 /
    90), the bell face 1904 flickering in the last 30 of each ring. Harry
    touching it while the bell face shows: `ShowBalloon` with
    `GeneratePhonewords`, mode 5; shown → 3101 (silent).
  - **Copier** 9200: Harry touching it with timer 0 sets its level entry
    to 9199 and calls `CopierSave` (the save game dialog); saved → the
    sprite becomes 9199 (used, no more saves), cancelled → timer 300.
  - **Magnet** 9300: Harry above it (his v + 32 < at.v + 12), less than
    96 px to the side (centres), with any ammo: every third frame (one
    counter for all magnets, `_DAT_1006c4ac`) sound 602, his speed v =
    2500, comfort − 5 unless on caffeine.
  - **Wall gun** 9401 up, 9411 down, 9421 right, 9431 left: fires while
    its switch is on - the switch is the level entry named by info #0
    (negative: on while its info #1 ≠ 0, else while it is 0) - or while
    its own info #1 ≠ 0; every 60 frames sound 250 and an enemy shot 9450
    at (at.h + 2, at.v − 12) / (2, 8) / (12, 2) / (−12, 2), speed 1500.
  - **Vending machine** 9600: entry info #0 = kicks until a can (2–4 when
    unset), info #1 = cans (2–4 when unset, −1 = empty). A kick
    (`HitPlayerSprite`) sets timer 3: kicked face for 3 frames, the first
    counts the kick; the last one with cans left plays 1302, resets the
    kicks and 25 frames later throws a can (enemy shot 9610 at (at.h +
    12, at.v + 28), speed v 1500–2299, h −1000…999); else sound 1301.
  - **Gate** 9800: once item 11199 is used on it (`HitPlayerSprite`,
    timer + 1), it loses its hit task, rises 1 px a frame (sound 700 at
    timer 3) and is gone after timer 64.
- `re/tools/emu_collider.py`: 20 000 cases (setup, up to 400 frames, the
  hit at a random frame, up to four sprites in the fan's list), all
  matching; 31 of 34 mutations caught, the other 3 equivalent (push 0 at
  the reach, the unsigned shift truncated to 16 bits, the centre line).

- **Enemy shots** (`SetupEnemyShotSprite`: layer 11, hot rect (0, 0, 8,
  8), fix from `pos`; kinds by shooter): darts 1018 / 1028 / 1038 / 1048
  (speed 3328 right / left / up / down, faces 3001–3004), the paper ball
  1051 (15-step spin, faces 3041–3048), homing shots 1101–1299, bolts
  2001–2999 (face 4801), 2281 / 2291 spinning (faces 5001… / 4981…), the
  gun shot 9450 (hot rect 16 × 16, face 3141), the can 9610; others
  (robots, turrets) a 7-frame animation (3021… below 1900, 3121… from
  1900). Kinds without a setup branch start with timer 8 and `f7c` 12.
  Each frame: `f7c` counts down; slower than 300 on both axes counts up
  `f78` (down when fast) and past 35 the shot is gone, as when `at`
  leaves 0…4096.
  - **Homing** (timer 0 - the shooter sets it): 8 faces 3081…; on the
    frame `f7c` reaches 1 it steers: `FindDesiredDirectionGeneric` from
    its centre to Harry's, impulse 50, at most 1200 (and `f6e` + 1, past
    100 it would burst - it steers only once). It bursts (timer −16) on
    a wall or another shot: no hit task, at − 4 and sound 4703, faces
    3101 + 8 − (−timer) / 2, gone at −2.
  - **Can** 9610: timer 45, 8 faces 3061…, from timer < 20 speed ×
    235 / 256, at 0 gone and a soda can item 11899 at at − 4.
- `HitEnemyShotTileSprite`: a wall (`WallBounce` with the shots' mask
  cicn 1299, half 4, bounce 255; foreground kinds above 55 count as hit):
  a homing shot bursts, the can bounces, others are gone. Its sounds are
  never played: the test is "kind < 2001 and kind > 3000". The floor's
  obstacles (`BGTileBounce`, bounce 240) only bounce the can. A shot gone
  this frame still meets the rest of the frame's tiles.
- `HitEnemyShotSprite`: the can `RectBounce`s (half 4, bounce 255) off
  colliders, coolers, plants, barrels, grates, chairs; other shots stop
  at colliders and sliding doors (sound 4701; the gun shot passes the
  guns 9401–9499); robots and grates take damage (with those kinds); a
  homing shot meeting an enemy or player shot bursts and stops it (or
  bursts it, if homing too).
- **`FindDesiredDirectionGeneric(to, from)`**: equal coordinates are
  moved apart (from + 1); the slope (to.v − from.v) / (from.h − to.h) in
  single precision is compared (in double) with tan 5°, 15°, … 85° as
  typed - 0.0875, 0.2675 / −0.2679 (the two sides differ), 0.4663,
  0.7002, 1.0 (a float), 1.4281, 2.1445, 3.732, 11.43 - per quadrant to
  give 0–35 (0 = left, 9 = down, 18 = right, 27 = up).
- `re/tools/emu_enemyshot.py`: 15 000 flights on all levels with the real
  tile pass, 12 000 hits, 30 000 aims (near the limits), all matching;
  all planted mutations caught except equivalent ones (the burst face
  formula, the never-zero slope sides) and two rare edges (exactly 4096,
  wall kind 55).

- **Dart enemy** (1011 / 1021 / 1031 / 1041 throwing right, left, up,
  down; layer 9, hot rect (8, 8, 24, 24), faces 2501 + dir + 7 × the
  direction). Only while `at` is in the view widened by 64 px up and left
  (`PtInRect` with (view.h − 64, view.v − 64, view.h + 512, view.v +
  512)): `f6e` counts frames - dir 1 / 2 / 3 / 4 / 5 at 1 / 4 / 7 / 10 /
  13, at 20 dir 6, sound 300 and `ThrowDart`, at 22 dir 0; other frames
  count down the timer (a hit's pause) or, with timer 0 and past frame
  45, start over with a 1 in 25 chance (`SATRand(25) == 2`). Layer 10 (hit)
  goes back to 7. `ThrowDart`: from `pos` + the hand of frame 6 (1011
  (23, 19), 1021 (1, 4), 1031 (20, 1), 1041 (4, 21) as (h, v)) a dart
  kind + 7 - or with the level entry's info #0 = 1 a paper ball 1051
  thrown forward at 1100–1699 with −250…249 across; info #0 other than 0
  / 1 throws nothing. A player shot hitting it stops (sound 4700), the
  enemy pauses 65–134 frames and Harry loses 3 corporate favor.
  `HitPlayerSprite` treats it like a collider (`RectBounce`).
- `re/tools/emu_dart.py`: 15 000 cases, all matching; 22 of 23 mutations
  caught (the survivor changes a hand offset of frame 1, which only the
  drawing uses).
- **Props** (`AddBGProp` / `AddFGProp`): object ids 1019 / 1029 / 1039 /
  1049 become background pictures (HrSp 6034–6037), 1609 / 1619 / 1629 /
  1639 foreground ones (6051–6054) - no sprite, just (point, face) in a
  table of 64 slots each, drawn when the point is in the view.

- **Ramps** (9701 / 9702 along v, 9703 / 9704 along h; layer 1, hot
  rects 9701 (8, 0, 60, 24), 9702 (0, 0, 52, 24), 9703 (0, 8, 24, 60),
  9704 (0, 0, 24, 52); faces 4101–4104, pushable 4105–4108). A ramp whose
  entry has info #1 ≠ 0 (four in Episode 1) moves by its speed each frame;
  all glide (`HandleGlide`, centre + 20).
- **`HandleRampCollision`** (sprite, half, hot rect, ramp, sound): nothing
  while the sprite's +0x94 is ≥ 1 (set to 3 by a bounce, counted down by
  `EraseSprites` each drawn frame, as +0x96). Positions from `fix`, the
  ramp's too. Coming across the ramp - its centre more than 8 px before
  the middle line and moving toward it - and not past the ramp's high end
  (13 px): put outside the ramp's side, the ramp gets half the speed
  (`>> 1`), the sprite −(bounce × speed >> 9). Else, past the low end by
  more than 15 px (on the ramp): 1 px toward the middle line (speed
  across halved, truncating) and +30 (Harry, kind 69) / +15 (others) up
  the slope. Else at the high end: moving into it faster than 450 →
  `air` 1 and the sound (209); slower → stop with its edge 16 px into the
  ramp. Else at the back end (within 6 px): bounce back. 9703 moves only
  `fix` at its back end, 9704 only `pos` - kept.
- **Pushable ramps** bounce with the `…2` routines, which take the
  centre as a point (v, h) - 9701 (34, 12), 9702 (26, 12), 9703 (12,
  34), 9704 (12, 26) - instead of one number; otherwise they are
  `WallBounce`, `BGTileBounce` and `RectBounceFake`, except that
  `BGTileBounce2` lets only a straight neighbouring wall (kinds 0–3) close
  a side. `HitRampTileSprite`: walls (`WallBounce2`, only with a face)
  and floor obstacles (`BGTileBounce2`) with the ramp's mask (cicn
  571–574) and the level's bounce. `HitRampSprite`: against another ramp
  `RectBounceFake2` with the other's hot rect placed at its *centre*, and
  this ramp's hot rect set to the other kind's (BUGS.md); unknown kinds
  use the centre (0, 0) from the data.
- `HitPlayerSprite`: a ramp, Harry on the floor and not kicking:
  `HandleRampCollision(Harry, 32, his normal hot rect, ramp, 209)`.
- `re/tools/emu_ramp.py`: 40 000 collisions (both kinds of rider, all
  sides, speeds at the 450 limits), 10 000 ramp frames, 12 000 ramp
  meetings, 12 000 pushed ramps on all levels with the tile pass, all
  matching;
  with the player hits (30 000) every mutation caught except two
  equivalent ones.

- **Whole frames** (`harry_core::game`): `Game::start` is `GameLoop`'s
  level start (`SetupLevelSprites`; Harry with `SATNewSprite(0, h, v,
  SetupPlayerSprite)` at the level's start − 32 and its start direction;
  the weapon; view and target at Harry − 208; one `FindUpperLeftCorner`)
  and `Game::frame` one pass: `HandleIdleSprites`, SAT's run (each task in
  list order, `hotRect2` = hot rect at `pos` right after it; new sprites
  go to the front and run next frame; the sort pass; the collision pass
  with both hit tasks; sprites without a task get kind 0),
  `CheckPlayerCollision`, `FindUpperLeftCorner`, `CheckTileCollisions`,
  `EraseSprites`, `UpdateMeters` (values clamped to 0–144 when one
  differs from its shown value, shown values one step closer) and
  `CheckGameEvents` (a changed level counter ends the level). Harry and
  his weapon are records; the routines ported with their own structs get
  a copy and write it back. `SetupPlayerSprite` takes `gPlayerDir` mod 35
  (not 36). `Ouch` halts the voices first, so it always draws a random
  number unless caffeine works. A sprite without a face makes
  `CheckTileCollisions` read face bounds at address 0 (low memory on the
  Mac; zeros in the recorder and the port). The data section starts with
  `gFloorKind` = 1 (and `gImpulse` = 16, which `SetupPlayerSprite`
  clears).
- `re/tools/emu_game.py`: 600 runs of up to 300 frames on level 1 with
  Harry's move keys held at random (start or next to a random sprite;
  the globals restored to the post-init data section per run), all 596
  comparable runs matching - list, free list, every record, Harry's path,
  view, meters, random seed. The other 4 kick the locked door 913, which
  reads stale stack values (BUGS.md). Not covered yet: weapons and items
  (player shots), explosions, death, the ripple, the frontend.
- **`HitPlayerSprite`, enemy shots** (on the floor): most stop on Harry;
  unless the ouch cooldown (`PTR_DAT_10065e44`) runs, `Ouch`, comfort
  delta and a share of the shot's speed: darts / paper (< 1100) −12,
  speed >> 4, cooldown 3; the can 9610 (not stopped) −7, >> 2, 12; the gun
  shot 9450 −10, >> 2, 12, sound 251; robot / turret shots −10, >> 4, 12,
  sound 223, no `Ouch`; a homing shot with timer 0 bursts (timer −16, no
  cooldown) −10, >> 4; 2281 favor −4, >> 5, 5; 2291 −7, >> 5, 5; other
  2001–2999 −4, >> 5, 3.

### Harry touches a sprite (`HitPlayerSprite`)

- Called by `CheckPlayerCollision`; nothing when Harry or the other
  sprite is frozen. One `else if` chain by the other sprite's task: ramp
  (on the floor, not kicking), stairs, desk, grate, door, sliding door,
  enemy shot, blob, robot, boss, reversor, mail guy, boulder, collider,
  then - for anything but a hazard - bonus, item, custom, effect, ghost.
  Ported with the kinds (`harry_core::touch`); `re/tools/emu_playerhit.py`
  records it with the real setups of the other sprites.
- **Bonus** (on the floor): 700–710 only while comfort < 144 - collected
  (frozen); 700 / 701 comfort + 10 (voice 210), 702 + 20 (212), 703 + 8
  (211), the voice only when none is playing; 704–710 vanish without
  effect. 711 / 712 / 713: staple / B.A.D.A.S.S. / shrapnel ammo + 20
  while below 100 (so up to 119), `HaltTalkingSounds` and sound 216.
  Full (comfort ≥ 144 or ammo ≥ 100): the bonus stays, and at most every
  200 frames (`t24`) a remark (218 comfort, 219 ammo).
- **Item** (on the floor): the staple gun, B.A.D.A.S.S. or shrapnel gun
  gives 50 ammo (up to 100) and vanishes, and goes into the inventory if
  Harry does not have it yet; anything else goes into the inventory
  (`GetInvItem`: a second one counts up) with sound 214 - and stays in
  the level when the inventory is full.
- 12 000 cases match; 12 of 12 mutations caught.
- **Door** (any `air`; nothing for open doors 902 / 912 / 922 / 932):
  walking into a closed or locked door: a locked one opens with its key
  selected (item 11399 − the door's info #0: sound 240, the key used up,
  door and level entry become the closed kind, +0x8a set), else a remark
  (205) at most every 25 frames (`t28`); then `DoorSeparate` (Harry back
  to his side, judged by his centre, from his fix position) and his speed
  reversed × bounce / 256. Kicking (hot rect left = 0): from the side the
  door opens to (his centre past the hinge: 901 v > door.v + 35, 911 v <
  door.v + 17, 921 h > door.h + 34, 931 h < door.h + 5 - using the
  `gPlayerH/V` copies) and facing it (901: dir 1–17, 911: 19–34, 921:
  10–26, 931: 0–8 / 28–35) the door swings open (its dir + 1, kick contact
  0, kick reset), from the other side facing it the kick hits it as solid
  (contact 256); both with sound 204. Otherwise `RectSeparate` with his
  normal hot rect (which stays for the rest of the frame). A locked door
  opens on a kick only with the key (which unlocks it either way).
- `RectSeparate`: the shortest of the four distances out of the other
  rect (ties: up, down, right, left) - Ghidra shows the minimum and the
  first distance as one variable, the machine code keeps them apart.
- 30 000 cases (bonus, item, door) match; 19 of 20 door mutations caught,
  the survivor changes a hinge coordinate the door's axis never reads.
- **Collider** (on the floor; not the phone 3100 / 3101; the same branch
  takes dart enemies, scientists and co-workers): once per frame (flag
  `PTR_DAT_10065e50`, cleared by `HandlePlayerSprite`) `RectBounce` with
  Harry's normal hot rect (`_DAT_10065e94`, (22, 22, 42, 42) whatever he
  is doing), half 32 and the level's bounce; a second such sprite in the
  same frame does nothing. Then: gate 9800 with item 11199 selected → the
  item used up, the gate's timer + 1 (it starts rising), score + 5000
  (`PTR_DAT_10065af8`); kicking (the kick rect of his dir at `at`
  touching the hot rect) the vending machine 9600 → its timer 3 if 0 when
  its middle is above his (mid v < at.v + 32), else kick contact 256;
  kicking the copier 9199 / 9200 → kick contact 256.
- 30 000 cases with colliders as well match; 14 of 15 collider mutations
  caught (the edge of the vending machine's middle after adding cases
  beside it); the survivor tests the top instead of the left of Harry's
  hot rect - the same in both of his hot rects.

### Sprite kinds (CODE 3)

Player (`HandlePlayerGlide`, `DoScoot`, `DoKick`, `DoInch`, `MoveChair`,
`HandleKeys`, inventory), barrel, blob, bonus, boulder, boss, button,
chair, collider, cooler, co-worker, custom, dart enemy, desk, door,
effect, enemy shot, exit, fire pipe, ghost, grate, hazard, held weapon,
item, mail guy, message (speech balloons), overhead, plant, player shot,
ramp, reversor, robot / machine gun guy, scientist, sliding door, stairs,
turret. Each kind has `Init…` (once), `SETUP…` (per sprite), `HANDLE…`
(per frame), `HIT…` / `HIT…TILESPRITE` (collisions).

## Data formats

### Level `Hlvl` (Episode 1: ids −1, 1–15; 76 168 bytes each, fixed size)

- 0x0000–0x7fff: **background grid**, 128 × 128 big-endian words,
  **column major**: tile (x, y) at (x · 128 + y) · 2 (`GetBGTile`).
  Values 0–14 are the random floor variants; higher values are special
  floor tiles (kinds: see "Tiles and collisions").
- 0x8000–0xffff: **foreground grid**, 128 × 128 words. 60–63 fill the
  area outside the level, 0xffff = no tile, 0–59 walls etc.
- Tiles are 32 × 32 → the world is 4096 × 4096 pixels.
- 0x10000–0x10fff: **sprite list**, 256 records of 16 bytes
  (`SetupLevelSprites`): byte 0 = present, word +2 = **object id** (the
  editor's ids: 0x0385 = 901 door up, 0x0259 = 601 plant, 0x23f0 = 9200
  copier, 0x01f4 = 500 water cooler, 0x2707 = 9991 fade exit, 0x5208 =
  21000 ?), words +4…+a = info fields, +c and +e = position (pixels);
  the sprite is generated at (+e − 6, +c − 6) with its record index.
- 0x11000: 16 records of 0x34 bytes, 0x11340: 16 records of 0x18 bytes -
  custom sprite tables (copied into the game state at level start and
  into saved games; `HitPlayerSprite` tests +0x0e == 3); empty in
  Episode 1.
- 0x114c0: **level name**, Pascal string ("Welcome to ScumCo"); also
  compared against fixed strings somewhere (cheat/special levels?).
- 0x12032: **level number**. `CheckWorldCorruption` compares number and
  name of each Episode 1 level with fixed values and quits with a dialog
  when they differ (anti-tampering).
- 0x12034 (byte): **own sounds** (`LoadLevelSounds`); set in level 10
  (water scoot sounds, `snd` 200/201/203 in Episode 1).
- 0x12074: **level info fields #0–#15**, words (see list below). Level 1:
  #6 = 200 (story picture scrolled before the level), #8 = 301 (picture
  shown before). Level 3: #5 = 201.
- 0x1209c–0x120b1: nibble arrays (`1111 1111 1111 1111 2222 2333`, `6666`).
- 0x120f4/0x120f6: **start position** v, h in pixels (level 1: v 2082,
  h 2087; `GameLoop` is called with h = 0x120f6 − 32, v = 0x120f4 − 32,
  and puts Harry's sprite there).
- 0x120f8: start direction (`GameLoop` sets Harry's `dir` to it; 27 = up
  in levels 1 and 12, else 0).
- 0x120fa: **music id** (601, 603, ...).
- 0x120fc: **foreground tile set** PICT id (its 1-bit mask is id + 1),
  0x120fe: **background tile set** PICT id; 0 = the defaults 181 (+182) /
  180 in Harry Graphics (`LoadLevelTilesets`; ids ≠ 0 are loaded from the
  world file).
- 0x12100: physics override flag; 0x12102–0x1210a: five physics values
  (defaults when the flag is 0: 50, 70, 245, 252, 150; `LoadLevelPhysics`).
- `crates/harry-original/src/hlvl.rs` reads all of this into
  `harry_core::Level` and writes it back byte-exactly (test over all 16
  levels). Still undecoded: 0x1210c (level 1: 80) and stale bytes after
  the name in levels −1 and 13 (an older, longer name).
- Level −1 is named **"Secret Level"** (default tile sets).
- Level names / tile sets (fg, bg): 1 Welcome to ScumCo (default), 2 To
  The Incinerator (128, 130), 3 Seekers of the Sun, 4 Robot Rebellion
  (default), 5 Below the Earth, 6 Depths of the Basement, 7 Forgotten
  Catacombs (131, 133), 8 Toil and Trouble, 9 Black Lab (134, 136), 10
  It's Cooler Here (137, 139), 11 Temple of the Swivel Chair (143, 145),
  12 Corporate Ruins (146, 148), 13 Military-Industrial Complex, 14 Armed
  to the Teeth (149, 151), 15 Confrontation (152, 154).
- `re/tools/render_level.py res <id> out.png [scale]` renders a level
  (both grids, tile sets with mask, sprite list as marked ids, start
  position); all 16 render plausibly (`re/work/png/levels/`). Object ids
  ≥ 21000 are memos (`21000` on the desk in level 1 = the first memo).

### World `Hwld` 0 (70 668 bytes) - to be decoded.

Level/world info fields from the editor's "Object IDs/Level Info":
level #0 player tint, #1/#2 fall location, #3 comfort after fall, #4 level
tint, #5/#6 PICT to scroll after/before, #7/#8 PICT to show after/before,
#9 scroll music, #10–#12 ripple length/width/speed; world #0 default
scroll music, #1/#2 PICT to scroll/show after win, #3 seconds, #4 win music.

### Tile sets (PICT in the world file)

Per set three PICTs: foreground colour (256 × 256 = 8 × 8 tiles of 32 px),
its 1-bit mask at id + 1 (the editor FAQ requires mask id = colour id + 1;
`pict.py` does not decode 1-bit pictures yet), background (256 × 384 =
8 × 12 tiles). Episode 1: 128–154 (9 sets), story/scroll pictures 200–209
(400 × ~1000 text), 301–315 (per level), 400. Each level names its
foreground and background set ("Level Info" in the editor).

### Screen palette

- The game installs **no palette of its own**: no `pltt`/`clut`
  resources; `SetEntries` only in the gamma fade code. SAT builds its
  offscreen GDevice with `GetCTable(8)` = the Mac's **standard 8-bit
  colour table** and a 4-bit inverse table (`gdResPref` = 4,
  `MakeITable`). So screen index i = clut 8 entry i
  (`re/tools/macpal.py`: 0–214 the 6 × 6 × 6 cube from white, 215–254
  red/green/blue/grey ramps EE…11, 255 black).
- `macpal.py` over all PICTs: 66 of 68 8-bit pictures use clut 8 exactly
  (pixel values are screen indices, nothing is mapped). Episode 1 PICT 145
  (a background tile set) has its own, reordered table, but all its 80
  colours exist in clut 8 → exact remap. Only Titles PICT 131 (Ambrosia
  logo, grey shades 254, 253, ...) has colours outside clut 8; QuickDraw
  maps them through the 4-bit inverse table (approximate). 11 PICTs are
  1-bit masks.
- Host screenshots cannot verify colours exactly (macOS scales them and
  applies the display profile: > 20 000 colours in one screenshot).
- Sprite faces (`HrSp`) and tiles are raw 8-bit indices → clut 8.

### Sprites `HrSp` (Harry Graphics, ids 1001–6054, 929 faces)

Named "Harry Sprite Face". Header: two Rects (top, left, bottom, right),
e.g. (0, 0, 44, 44) twice; then 32-bit commands (high byte opcode, low 24
bits n), as `BlitEncFaceNoClip` reads them: 0 end, 1 next row (n = the
row's encoded length, unused), 2 copy n pixels (data padded to 4 bytes),
3 skip n pixels (transparent). Pixels are clut 8 indices.
`re/tools/hrsp.py` decodes all 929 to RGBA PNG (`re/work/png/hrsp/`):
1001… inventory items, then objects in 36 rotations. Ids in groups per
object kind (cf. editor object ids).

### Sound `snd ` (Harry Sounds 100, Episode 1: 200, 201, 203)

Format 1 with one bufferCmd, 8-bit unsigned: 54 at 22 050 Hz, 46 at 11 025
Hz, 2 at 22 254 Hz; one format 2 at 5 564 Hz. Played through SAT's sound
queue plus `STPlay3DSound` / `CalcStereoVolume` (positional stereo).

### Music `Tune` (Harry Music 600–620)

QuickTime Music Architecture tune sequences (`TuneSetHeaderWithSize`,
`TuneQueue`, `TuneTask` imported by the PPC code), played with the
instruments from `Harry Instruments` (35 `ssai` sampled instruments).
600 intro/theme, 601 levels 1–2, 603–615 levels 3–15, 616 ending, 620
storyline. Custom worlds may import MIDI files (editor b4,
`ImportMovie`). For the port the tunes must be rendered with these
instruments (a small QTMA synth) or recorded from the emulator.

### Pictures (Harry Titles)

128 Loading Screen, 129 Game Screen (640 × 480: play area ~450 × 450 on
the left, panel on the right with LEVEL, SCORE, COMFORT, SURFACE
RESISTANCE, CORPORATE FAVOR), 130 Menu Screen, 131 Ambrosia Logo, 132
Instructions, 4803–4921 messages/buttons.

## Emulator

**Status (2026-10-23):** the 68k build runs in Basilisk II (System 7.5 +
Sound Manager 3.1): shareware notice, main menu, preferences, level 1,
scoot, memo balloons. Screenshots in `re/work/screens/`. The PPC build
runs in SheepShaver (Mac OS 9.0.4) as well. At first it quit with
"Couldn't open Sound Tool!" (its mixer `FUN_10056590(8 voices, stereo,
22050 Hz)`: Gestalt 'snd ', `SndNewChannel`, `SndPlayDoubleBuffer`
failed): after a fresh Mac OS 9 install **no sound output device is
selected in the Sound control panel** - select it (the user did) and the
game runs. Level 1 looks identical in both builds.

Preferences seen in the original: Game Options (Allow system tasks, More
compatible drawing, Reduce frame rate, Line-skipping + Interlace skipped
lines, Save between levels, Camera Mode: Constant Catch-up / Constant
Anticipate / Step-Scroll Smoother (default) / Step-Scroll Faster), Key
Controls, Sound & Music. Start-up: Ambrosia logo, shareware notice ("Not
Yet" / "Register..."), main menu (New Game, Continue Game, Instructions,
Options, Credits, Quit) over the wire-frame chair; New Game scrolls the
story (PICT 200) in the play area, a click skips it. A memo balloon dims
the play area.

### SheepShaver (PPC)

`re/work/emu-ppc`: SheepShaver built from kanjitalk755/macemu (`src/`,
`--disable-jit`, SDL2), ROM `newworld86.rom` (user), Mac OS 9.0.4 CD
(user, `691-2684-A`, linked as `macos904.iso`) installed onto
`macos9.dsk` (2 GB, Mac OS Extended; "Update Apple Hard Disk Drivers"
switched off; clean copy `macos9-clean.dsk`), Harry installed by running
the installer again under OS 9 (puts Harry Instruments into its System
Folder), `harry.dsk` = copy of the Basilisk disk. `sheepshaver.prefs`:
256 MB, 640 × 480, `bootdrive 0` (−62 for the CD). Start from the
repository root: `cd re/work/emu-ppc && src/SheepShaver/src/Unix/SheepShaver
--config "$PWD/sheepshaver.prefs"`. It runs at full speed for the user;
the slow input during my installation was the installer / first boot
(or my automation), not the emulator in general. Once (2026-10-23) it
quit silently during play (no crash report, no log message) - right
after test key events (Shift + = + Page Up via Quartz); cause unknown.

### Basilisk II (68k)

`re/work/emu`: Basilisk II from the S.C.Out work folder (binary used in
place), Performa ROM, a copy of the System 7.5 + Sound Manager 3.1 boot
disk (now with `Harry Instruments` in Extensions), `harry.dsk` (100 MB
HFS) with the installer, `harry-installed.dsk` = after installation.
64 MB RAM, 640 × 480 × 8. Clicks must be press - pause - release
(cliclick `dd`/`du` with 100–150 ms), instant clicks are ignored.
The boot disk has no QuickTime yet: music needs QuickTime 2.5+ with
QuickTime Musical Instruments.

## Port and differential tests

- Rust workspace: `crates/harry-core` (game logic, no I/O; doc comments
  name the original routine), `crates/harry-original` (tests against the
  original). `cargo test` runs them; tests that need recordings skip when
  `re/work/diff/` is missing.
- `re/tools/emu_physics.py` runs `ApplyModedImpulse`, `EnforceMaxSpeed`,
  `HandlePlayerGlide` and `MoveChair` (with `DoScoot`, `DoKick`,
  `DoInch`; `STPlayRegSound` stubbed and counted) of the PPC build in
  Unicorn on 4000 random states each (fixed seeds) → `re/work/diff/*.txt`.
  The port matches all 16 000 cases. Planted bugs (a sine digit, 236 →
  235, rounding instead of truncation, the 25 threshold, kick factors,
  the missed-kick push, a dropped sound, an inch value) are all caught;
  two surviving mutations were equivalent (`max <= h`, `p * 5 / 2` for
  the caffeine factor).
- `re/tools/emu_bounce.py`: `WallBounce` on 12 000 cases (all kinds,
  first-contact flag, bounce factors; the region mask test `FUN_10054124`
  stubbed with its result as an input). `harry_core::bounce::wall_bounce`
  matches all; 11 planted bugs (corner swap, diagonal choices, clamps, the
  9 px threshold, kind 0's first-contact rule, ...) are all caught.
- `RectBounceFake` and `BGTileBounce` ported and tested the same way
  (12 000 cases each). `BGTileBounce` runs on the real levels 1–15
  (`ppcmachine.Machine.load_level`: the Hlvl as the handle `gLevelH`,
  then `LoadTileDefinitions`), at background obstacles and free tiles next
  to walls; the four neighbouring foreground kinds are part of each case.
  Its quirks: speed damped twice (when the side is chosen and again on the
  frame's first contact), the side moved away from a neighbouring wall
  (kinds 1/6/5 above, 2/7/6 right, 0/4/5 left, 3/7/4 below), position
  only clamped to 14 px from the tile centre.
- Sprite record: +0x5a/+0x5c (`atV`, `atH`) are not centres but a working
  copy of the position (top-left of the 64 × 64 face) that the collision
  code moves; the centre is at + 32 (`half`).
- The region mask test (`FUN_10054124`): mask regions of sprite face and
  tile, offset to their positions, `SectRgn` / `EmptyRgn`; on overlap
  `FUN_10053eac` (68k `REGIONSEPARATE`) moves `pos` one pixel at a time
  along a Bresenham line from the intersection's bounding-box centre to
  the sprite region's centre until the regions no longer overlap.
- **Collision masks** (`InitFaces`): wall shapes = application PICT 160
  (1-bit, 256 × 128 = 32 tiles of 32 × 32, 8 per row; colour and mask
  from the same PICT, `FUN_10054a14`), background obstacle = cicn 290
  (32 × 32 rounded square), Harry's body against walls = cicn 299 (a
  small circle in the middle of the 64 × 64 frame; passed by
  `HitPlayerTileSprite` to `WallBounce` / `BGTileBounce`), barrel /
  cooler / plant masks cicn 596–599. Face sets get regions from their
  pixels (`BuildMaskRegion`: pixel ≠ 0 → set, see BUGS.md for the bit
  order) only for the frame range given to `LoadEncFaceSetFromPICT` -
  for Harry 107–143 (the kick poses).
- `re/tools/emu_mask.py` runs the mask test with the real masks: QuickDraw
  regions are emulated as pixel sets (`ppcmachine`: `NewRgn`, `CopyRgn`,
  `OffsetRgn`, `SectRgn`, `EmptyRgn`, `DisposeRgn`, with `rgnBBox` kept
  in memory because the game reads it). 12 000 cases (body vs. all 32
  wall shapes and the obstacle), 5 626 hits with separation:
  `harry_core::mask::mask_hit` matches all. Mutations caught (direction
  tie, Bresenham comparisons - one of them loops forever); `>> 1` vs
  `/ 2` for the centres is equivalent for the game's positive
  coordinates.
- Rust readers (`harry-original`): `pict` (PackBits 8-bit pixmaps with
  colour table, 1-bit bitmaps - rows under 8 bytes unpacked -, and
  `DirectBits` 32-bit packType 4: application PICT 140), `cicn`, `hrsp`,
  `masks` (body / obstacle from cicn, walls from PICT 160, face masks with
  the original's mirrored bits). Tests: all 82 pictures decode; the 929
  faces equal the Python decoder (size, opaque pixels, index sum); the
  masks equal the recorded ones; face masks equal the original's
  `BuildMaskRegion` run in Unicorn (`emu_mask.py re/work faces`, GWorld
  stubs in `ppcmachine`: `NewGWorld`, `GetGWorldPixMap`, `EraseRect`,
  `BitMapToRegion` MSB first, ...).
- **One frame of tile collisions** (`re/tools/emu_tiles.py`): the
  original's `CheckTileCollisions` with Harry as the only sprite in SAT's
  list (`*(SAT globals + 0x78)`, next at +0x2e) and his real
  `HitPlayerTileSprite` (TVector 0x10066fbc) - all bounces, mask tests,
  kick tests with the kick-pose masks, floors, damage, water - on levels
  1–15 with the real masks and the game's tables; sounds / `Ouch` /
  `AddFGRedraw` stubbed. 19 500 cases (15 000 at obstacles and wall
  edges, 4 500 on special floors with wetness around the water sound's
  threshold). `harry_core::tiles::touched_tiles` +
  `harry_core::player::hit_player_tile` match all of them; the kick
  rects and tile shapes are also compared with the tables the original
  computes. Mutations caught, including one pixel of a kick rect and the
  water threshold (after adding the targeted cases).
- Player tile reactions in detail: while kicking (hot rect left = 0)
  walls and obstacles are not bounced off: the foot rect of the
  direction (`InitPlayerKickRects`, on the side opposite to the direction
  of travel) is tested against the tile, then the kick-pose mask against
  the tile mask; on contact `gKickContact` = 256 and Harry is put back
  (the separation is undone). Damage tiles do not hurt while kicking.
  The water sound plays only when `gWet` < 171 (and it is not playing).
  The wall-shape table holds 32 faces, but foreground kinds up to 54 index
  it - out of range for kinds 32–54 (never used in Episode 1, where those
  tiles have kind −1).
- **Jump state** (`sp->air`, +0x8e - earlier mislabelled "hurt"): 0 on
  the floor; 1–35 a jump (scale +0x10 … +0x50 … 0, a second smaller hop
  23–35; sounds at 23 and 29; moves blocked from 23 on); 215–220 the
  flip when a half pipe throws Harry over its edge (speed forced to ±0x200
  in the flip direction, scale pulsing every 4 frames); 999/1000–1045
  falling into a hole (background kind 7): shrink, speed × 0.8 per frame
  and spinning until 1029, at 1030 back at the level's fall location
  (info #1, #2) ± 3 px from `SATRand(7)` twice, view recentred, then
  growing and spinning, at 1045 comfort += info #3 and air = 10 (a small
  landing jump). Half pipes (background kinds 8–15, levels 7, 12, 14)
  push toward their middle (Harry 0x90 / 0x30 + 3 · distance, other
  sprites 0x60 / 0x20 + 2 · distance) and throw the sprite off (air 1)
  when it leaves through the far side; holes are in levels 11 and 15.
- `re/tools/emu_air.py`: `HandleHalfPipe` and `JumpScale` on the real
  levels (23 000 cases each, weighted to the levels with half pipes and
  holes; `SATRand` with a recorded seed; fades stubbed; the view globals
  recorded). `harry_core::air` (+ `harry_core::random`, the QuickDraw
  generator) matches all. Two mutations survived as equivalent (`× 0.8`
  vs `× 4 / 5` on integers; `< 0x40e` vs `< 0x40d` where 1037 is odd).
- **Harry's frame** (`HandlePlayerSprite`, keys stubbed):
  `re/tools/emu_frame.py` records it on the real levels with all globals
  around the player set at random (20 000 cases): position sanity check,
  flags cleared, comfort delta (not while caffeine works), 15 timers
  counted down in a fixed order, idle chatter (`SATRand(100) < 30` when
  the chatter timer runs out, then `SATRand(3)` picks the voice), the
  inventory check, `MoveChair`, `HandlePlayerGlide` (sets the surface
  meter), `HandleHalfPipe`, the direction wrap (≥ 36 → 0, < 0 → 35),
  the face (`dir + pose` before `JumpScale` may spin him), `JumpScale`,
  wetness drying (−2 at speed ≥ 201), max speed 2500, floor kind reset,
  position update, shake (`SATRand(6)` − 3 on h, then v), death when a
  shown meter is below 1 on the floor. `harry_core::frame::handle_player`
  matches all cases; surviving mutations only in unreachable states (dir
  ≥ 37, negative air after `JumpScale`).
- **Keys** (`HandleKeys` with `GetInvItem`, `SwitchInvItem`,
  `RemoveInvItem`; their drawing stubbed): `re/tools/emu_keys.py`, 30 000
  cases with random key sets (default and random bindings, the fixed
  keys), inventories, ammo, charge times, sequence steps, the debug flag
  and the "is a sound playing" answers as inputs; sounds, explosions,
  balloons, the new player shot sprite and `BuildSineTable` recorded as
  an event list (the `snd` ids put into the sound slots, the channels the
  routine keeps marked as charge / voice). `harry_core::keys::handle_keys`
  + `harry_core::inventory` match all. 29 of 30 planted mutations caught;
  the survivor (`cur_slot < 0` for `== -1`) is equivalent. Details:
  - Order: debug keys, size Easter egg, rotate (+0x10 → dir + 1, +0x12 →
    dir − 1), select item (+0x1a, only when `gInvSwitchDelay` is 0; else
    the keys 1–4, which ignore the delay and pick the last slot with the
    item), the use key, the three key sequences, then the moves: kick
    (+0x18) whenever no move runs, scoot and inch only on the floor (air
    < 1).
  - Use key held: staple gun fires at once (−1 staple, 3-D sound pitch
    62000 + `SATRand(6000)`, fire delay 10); B.A.D.A.S.S., shrapnel gun
    and the power of the swivel count `gFireHold` up (the first frame
    starts the charge sound); caffeine pill (gulp, `gCaffeineTimer` 270,
    item use delay 8); soda can (shot kind 4, delay 6); the mystery vial
    - ignoring the fire delay - an explosion on Harry and
    `SATRand(6)`: 0 caffeine, 1 / 3 comfort = 100, 2 comfort − 15 with
    the "uncomfortable" sound, 4 all ammo 100, 5 all ammo − 25 (no lower
    bound).
  - Use key released with `gFireHold` > 0: B.A.D.A.S.S. weak (< 34
    frames, −2) or strong (−6), shrapnel kinds 0 / 5 / 6 / 7 by hold < 8
    / 16 / 24 / more (costs 1, 1–2, 2, 2–3), fire delay 4; anything else
    (the swivel): hold ≤ 20 nothing, comfort < 10 the "uncomfortable"
    sound, else comfort − 2 and a player shot sprite (kind 10) at Harry
    with speed 2000 against his direction (`InverseMode`); hold becomes
    10 (then 0 the next frame).
  - The shot itself: firing sets `gRecoil` = 3 and `gShotKind`;
    `HandleHeldWeaponSprite` calls `MakeNewShot` while `gRecoil` > 1.
  - `gFireHold` is the word right after the inventory table (slot 12);
    `SwitchInvItem` reads it as `inventory[12].id` when slot 11 is
    selected (harmless).
- **Player shots** (`re/tools/emu_shots.py`; `harry_core::shots`):
  - Firing sets `gShotKind` and `gRecoil` = 3 (`HandleKeys`); the frame
    counts the recoil down, and `HandleHeldWeaponSprite` calls
    `MakeNewShot` while it is > 1 - one shot (set) per firing, the frame
    after the key.
  - The held weapon sits at Harry's top-left (scaled with
    `FindScaledPosAndBounds`, which gets scale0 for both axes) + a
    per-direction offset (`WEAPON_OFFSETS`) × scale − 6; its face is
    `gPlayerDir` (+ 36 while recoiling), gun 0/1/2 for staple gun /
    B.A.D.A.S.S. / shrapnel gun, none for other items.
  - `MakeNewShot` from the weapon position + 2: kinds 0 (staple), 1 (weak
    B.A.D.A.S.S., speed ± 512 random), 4 (soda can) one shot; 5/6/7 two /
    three / four shrapnel pellets (kind 0, position −4…+4 px, speed ±256);
    2 four B.A.D.A.S.S. pellets in the four quadrants, or with no ammo the
    backfire (explosion at the weapon − 26 px, sounds 602 and 239).
    Speeds from `AssignShotSpeed` (`SHOT_SPEEDS`, 2200 in the facing
    direction, captured from the original).
  - Shot sprites: hot rect (−2, −2, 10, 10), kind 10 (the swivel shot,
    drawn as a spinning Harry, dir + 2 per frame) (27, 27, 37, 37); timer
    3 frames before walls count; animation 0–6; straight flight.
  - Walls (`HitPlayerShotTileSprite`): foreground tiles only, the small
    rect test (with a bug, BUGS.md), then `WallBounce` (half 4, bounce
    factor 1, not first contact) with the shot's mask (face 4021, mirrored
    like all face masks). Hit: `frozen`, sound 4700 (3-D), the soda can
    explodes (sound 500, explosion at − 28 px).
  - 20 000 weapon cases (scales including the garbage scale, random face
    bounds), 5 000 shot flights, 22 500 shot / wall cases on levels 1–15:
    all match; 30 planted mutations caught.
  - Not yet: shots hitting sprites (`HitPlayerShotSprite`, by the other
    sprite's kind - with those kinds).
- Full chain `WallBounce` + mask test on the rebuilt mask tables (walls at
  `*_DAT_100658d4` + 0x1a + kind · 0x2e, obstacle at `*_DAT_100658d0` +
  0x1a): 12 000 cases for the mask kinds 4–31, all matching.
- Harness notes: Unicorn's PPC needs a CPU model with FPU
  (`UC_CPU_PPC32_750…`) and MSR[FP] set, else the first float instruction
  raises an exception. The physics array is bss, filled by
  `LoadLevelPhysics` at level start - tests must set it.
- The PPC code computes `speed ± k · p` with separate `fmul` and
  `fadd`/`fsub` (no fused multiply-add) in double, then `fctiwz`: Rust
  `f64` arithmetic and `as i32` give the same results.
