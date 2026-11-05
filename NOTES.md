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

- The PEF (data fork, `re/work/ppc/game.pef`): 3 sections - code 414 604 bytes, data 35 168
  (packed 17 167), loader 5 504. Ghidra's PEF loader with **processor
  `PowerPC:BE:32:default`** (auto-detection picks a 64-bit VLE variant -
  wrong). Code loads at 0x10000000. Ghidra's `PefDebugAnalyzer` aborts
  (read at 0x1006538c), so it creates only the functions reached by
  direct calls.
- **Traceback tables** (AIX style, after every routine: zero word, fixed
  part, `tb_offset` = routine length, name) give exact starts and names:
  the skill's `scripts/ppc/pef_traceback.py` → 505 routines. `ApplyNames.java`
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
  → `re/work/ppc/game.c`, 1043 functions). It calls the classic-mac-port
  skill's `scripts/ppc/run_ghidra.sh` (skill folder: `CLASSIC_MAC_PORT`,
  default `~/.claude/skills/classic-mac-port`; it picks the newest
  Homebrew JDK, Ghidra 12.2 needs 25) with Harry's lists: `names.txt`
  (SAT routines without traceback), `globals.txt`, `sprite.txt` and the
  name pattern of the sprite routines. The scripts, in order:
  - `ApplyNames.java`: functions and names from `traceback.txt`.
  - `NopTOCRestore.java`: CodeWarrior reloads r2 after every call
    through glue (`lwz r2,0x14(r1)`, 2522 times); Ghidra then loses the
    TOC and shows `unaff_r2 + -0x78cc`. The script replaces these loads by
    nops *in the Ghidra database only*.
  - `SetTOC.java`: r2 = TOC = **0x1006d390** over the whole code block
    (also written to `toc.txt`).
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
  global (there: `gImpulse`, r31 = r2 − 0x48fa); check with the skill's
  `scripts/ppc/ppcdis.py` (switch tables: `ppc_switch.py`).
- The Unicorn harness `re/tools/ppcmachine.py` is the skill's
  (`scripts/ppc/ppcmachine.py`) plus Harry's addresses and `load_level`.
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

### Main menu (`MainMenu`, `DrawMainMenu`)

- Fade out (8), PICT 130 (Titles) on the whole screen, fade in (8),
  music 600. Under it a line in Helvetica 9 bold (black, grey, white,
  offset by a pixel each) centred over the game area at y 468: "This copy
  of Harry is unregistered. Please support shareware by registering." or
  "This copy of Harry is licensed to: <name> (<n> copies)"; in debug mode
  "CHEAT MODE ENABLED" at (64, 64). The port shows no line (it plays as
  registered and has no name).
- Six buttons at x 492–602: New Game (y 44), Continue Game (75),
  Instructions (106), Options (137), Credits (168), Quit (200), each 26
  high. A click is hit 1 px inside (`PointInRect`), then
  `TrackClickOnCommandButton(n, 1)`: pressed PICT 4900 + n and `snd`
  4803 (volume 0x55) while the mouse is on the button, released PICT
  4910 + n and 4804 when it leaves; after more than 9 presses in one drag
  `snd` 199; the action when released on it. Keys (`IsPressed`, every
  pass): N, O or R, I, P, C, Q - the pressed picture, 4803, `Delay(10)`,
  the action (the button stays drawn pressed). X: `snd` 206 and a message
  ("But Mr. Horrible says, …"), not again for 30 passes.
- New Game: `NewGame` (Episode 1 at level 1, with Option and debug mode
  the warp dialog) → `GameLoop(…, intro 1)`. Continue: a file dialog
  (`ContinueGame`). Instructions: the game area black, PICT 132 (403 ×
  396) at (38, 41) - on the window and the back buffer; it stays until
  something draws over it, the buttons go on. Options: the preferences
  dialog (DITL 6900–6903). Credits (`FUN_10055f90(600, 4803)`): DLOG 600
  (200 × 285, centred), PICT 4804 in it, PICT 4803 (136 × 988) moving up
  a row every `Delay(3)` through (34, 120) – (170, 270) of the window,
  from below, over and over, until a click in the window (a click beside
  it beeps). Quit: `Quit`.
- In the game (`CheckGameLoopKeys`): Escape asks "Are you sure you want
  to abort this game?" (`ConfirmAbortGameDialog`), yes ends the loop;
  Command-Q asks to quit, Command-W the same as Escape, Command-S
  `DoSaveGame`, Command-I the options; caps lock pauses; the map key (prefs +0x1e,
  Tab) shows the map (`ShowMap`, below), then not again for 6 passes.
- The port (`crates/harry/src/menu.rs`): the menu, buttons by mouse and
  key, the instructions, the credits, the X message (in a box of the
  port's), Escape in the game with a question box (Return / Y yes,
  Escape / N no), the options screen ("Port improvements"). Not yet:
  Continue from a list of saves (later; for now the game saved in this
  session), the warp dialog.
- **Pause** (`Pause`, from `CheckGameLoopKeys` when `CapsLockDown`): sound
  198 (`STPlayRegSound(…, 100, 0x100)`), PICT 4921 (225 × 96, "PAUSED -
  Press CAPS LOCK To Resume") drawn at (128, 192) over the game area,
  the cursor and menu bar shown, then `CheckEvent` until caps lock is up
  again (the music goes on); then the game area is redrawn and the loop
  goes on. **Command-I** (key 0x22 with Command): `Options()`, the
  volumes applied, the game area redrawn. The port: both, Command-I
  (Control-I off the Mac) opening the options screen over the game area
  (the game waits; Escape or Done goes on).

### The map (`ShowMap`)

- Drawn on the back buffer over the game area, the game waits: the area
  grey (RGB 0x451e), then 112 × 112 tiles of 4 × 4 px - the middle part
  (from tile (8, 8)) while Harry (`gPlayerH/V`) is within 0x500 … 0xb40 on
  both axes, else the quarter he is in (from 0 or 16 on each axis). A
  tile not on the track map is black; a seen one with a foreground tile
  below 32 gets its map tile (PICT 170 in `Harry Graphics`: 32 faces of 4
  × 4, `LoadEncFaceSetFromPICT(170, 32, 4, 4, 8, …)`), from 56 on black,
  else (none, 32–55) the grey. Harry: cicn 200 (32 × 32, a red cross) at
  ((H + 32) / 8 − 4 · from_h, (V + 32) / 8 − 4 · from_v) - its middle on
  his tile. Every 2 ticks the map key is read; after 6 reads it closes the
  map, the game area is drawn again.
- The port: `render::draw_map`, opened from the game loop as
  `CheckGameLoopKeys` does (`HARRY_MAP=1` / `all` in the screenshot mode
  shows it).

### Between the frames: fades and pictures

The game loop stops for these; they run in ticks (1/60 s) and draw on the
window. The port plays them in the frontend (`crates/harry/src/scenes.rs`,
tested for their timing).

- **Gamma fades** (`GammaFadeOut(n)` / `GammaFadeIn(n)`, through the
  gamma tables): to black in levels 0, s, 2s … ≤ 256 with s = (256 / n) /
  2, one a tick, then 256 at once; back with s = 256 / n. Neither does
  anything when the screen is already there (`_DAT_100658f4`). n = 8: 17
  ticks out, 9 in; n = 75 (`Win`): 257 out, 86 in.
- **`EndLevelEffect`**: the game area in 14 × 14 cells of 32 px; each has
  a counter, the top half (rows 0–6) starting at −(row · 14 + column), the
  bottom half from the bottom right (row 13, column 13 = 0, then −1 to
  the left and up). Every tick all counters below 17 grow by 1 and a black
  `FrameRect` of half size c is drawn around each cell's centre while 0 <
  c < 17 - growing black squares from the top left and the bottom right,
  114 ticks. After `UpLevel`'s jingle; at the end of `GameLoop` when
  comfort or corporate favor (the values) is below 1.
- **`ScrollPlot(pict, music)`**: the world's PICT drawn into a GWorld 400
  wide, then rows top … top + 448 copied to (40, 16) – (440, 464), top from
  −447 to the picture's height (h + 449 steps of 4 ticks: 15 rows a
  second; PICT 200, the story, 96 s). Rows outside the picture are not
  copied: the window keeps what it had there. Any key or the mouse
  button down ends it once 8 steps are shown (the port: only a press
  made during it; "Port improvements"). Then the game area is painted
  black. With `music` the world's scroll music (Hwld +0x144) plays.
- **`IntroLevel`**: fade out (8), the game area black, the level's title
  card - PICT level info #8 (Hlvl 0x12084), drawn centred across the game
  area but 16 px too high ((448 − w) / 2 + 16, (448 − h) / 2) - or, without
  one, the level's name (Hlvl +0x114c0) in Helvetica 27 bold over two
  lines and "L E V E L" with the number (+0x12032) in 11 pt (the port draws
  no text card yet; Episode 1 has a picture for every level), then
  `BuildSineTable`, fade in (8), `Delay(60)`.
- **`GameLoop`'s start** (`intro` = 1 for a new game and a game continued
  at a level's start, 0 from a copier save): the screen PICT 129; with
  `intro` and a story before the level (info #6): fade in (8),
  `ScrollPlot(info #6, music)`, fade out (8); then `IntroLevel`.
- **`UpLevel`**: jingle 4800, `EndLevelEffect`, then past the world's last
  level `Win`; else the story after the old level (info #5) scrolled, the
  game area black, the new level, `IntroLevel`, and only then its story
  before (info #6, after painting the area black) - the other order than
  at `GameLoop`'s start.
- **`Win`**: the world's win music (Hwld +0x14c), `ScrollPlot(Hwld +0x146)`
  without music, fade out (75); if Hwld +0x148: the whole screen black,
  that PICT at its own frame (`FUN_1005dacc` mode 0), fade in (75), Hwld
  +0x14a seconds (music idling), the music fades, fade out (75). Episode
  1: 616, 209, 400, 12 s. Then the main menu.
- The pictures are the world's (Episode 1: stories 200–209, 400 wide and
  424–1309 high; title cards 301–315, about 49 high; PICT 400, 640 × 480),
  all in clut 8; the data pack keeps them in `world/` with the world's
  fields in `world.txt` (pack format 3).

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

Drawing (`UpdateMeters(all)`: 0 = all, once at `GameLoop`'s start; 1 every
pass, only meters whose shown value differs): each meter is drawn with
its shown value **before** the step toward the value. From PICT 150 (147
× 44): the empty strip (row 0) up to shown + 3 (all: 147), then the
value's strip (row 11 / 22 / 33) from x 473, `shown` px long - 3 px
shorter once shown > 8, where the rounded end, cicn 10135 / 10136 /
10137 (9 × 9, mask 3 px wide), is plotted at (473 + length, y + 1)
(`FUN_10062a0c`). The shortening is one variable for the whole call
(BUGS.md). The screen keeps the picture until the meter is drawn again;
the port keeps what was drawn last (`Game::meters`) and the frontend
draws from it. The whole-frame recorder logs the strips (`CopyBits` from
rows 11 / 22 / 33) and the ends: all runs match.

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

### The inventory on the panel (`UpdateInventory`, `UpdateAmmo`)

- 12 slots, 3 columns × 4 rows, 44 × 44 at (476 + 49 · col, 167 + 49 ·
  row) (`FindCol` / `FindRow`, the same as `FindInvCol` / `FindInvRow`).
  `UpdateInventory` draws each slot that holds an item: the selected one
  (`gCurInvSlot`) on the highlight HrSp 1002 (cicn 11000) at + 4, the
  others on black; the frame HrSp 1001 (cicn 10200); the item's face at
  + 4 (`FindInvItemFace`: 11398 → HrSp 1031, 11397 → 1032, else 1010 +
  (id − 11199) / 100, from cicn 11100 + 100 k); with more than one "x"
  and the count in white, bold 9, at (+ 6, + 38) and (+ 13, + 38)
  (`DrawNumItems`). `RemoveInvItem` puts HrSp 1040 (cicn 10300, the
  empty slot) over the last slot and the removed one, moves the rest up
  and draws again. `SwitchInvItem` / `GetInvItem` redraw the slots
  concerned. Each calls `UpdateAmmo(0)` after.
- `UpdateAmmo(redraw)` (also every pass of the game loop with 1): the
  ammunition of the staple gun, the B.A.D.A.S.S. and the shrapnel gun
  clamped to 0–100 - a game state change; then for each gun in the
  inventory (the last slot holding it) a bar under it at (slot + 5,
  slot + 34), 34 × 5: the first ammo / 3 px light (0xAAAA grey, top edge
  white, bottom 0x7777), the rest dark (0x5555, top 0x1111, bottom
  0x9999); the ends and the boundary as vertical lines. With 1 only when
  the value changed (`_DAT_10065a98` / `…94` / `…90`).
- The port draws the inventory every frame from the state
  (`render::draw_inventory`; the screen of the original keeps what was
  drawn, which is the same); the core clamps the ammunition after
  `UpdateMeters` as `UpdateAmmo(1)` does, and the whole-frame recorder
  runs `UpdateAmmo` (with ammunition outside 0–100 in some runs).

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
  both `ShowBalloon(NULL, text, 2)`: over Harry, no sound, nothing frozen.
  The texts (insider jokes; Ben's says he wrote the game) exist only in
  the PowerPC data section (offsets 0x3818 / 0x3718; the 68k build keeps
  them in its compressed `DATA` 0): `harry_original::pef` unpacks PEF's
  pattern-initialised data (checked against Ghidra's: the 3003 words that
  differ are all the loader's relocations) and the pack keeps them in
  `messages.txt`. The whole-frame recorder types both sequences in some
  runs.
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
  kind 9450 also pushes it (speed / 4). Barrels and chairs count as
  plants (barrel: cicn 598, chair: cicn 599; both trade momentum); ramps:
  `HandleRampCollision` (centre + 18, sound 1101 when it takes off).
  (The port missed the colliders' `RectBounce` until the recorder had
  colliders.) Desks: always a hit (no mask test; jittered like the
  others), sound 1100, the plant bounces back × −0.4 like from a cooler;
  the jitter block also sets the desk's pos / at from its `fix`, which
  the desk's setup never sets (BUGS.md).
- `RectBounce` (PPC 0x1001e4bc): if the hot rects (at `at`) overlap, the
  side is chosen from the centres (h distance against v distance), the
  sprite put just outside, and the speed across reflected (both × bounce
  / 256) only when it moves into the rect.
- `re/tools/emu_planthit.py`: 30 000 cases (Harry kicking / walking,
  plant, cooler, all door kinds, shots, explosions, barrel, chair, ramps,
  colliders), all matching; 15 of 16 mutations caught - the survivor
  shifts the jitter by 1/256 px, which the mask separation right after
  almost always overwrites; the five later ones (barrel mask, momentum,
  ramp, collider, ramp sound) all caught.
- **Barrel** (550; `harry_core::furniture`; layer 6, hot rect (0, 0, 24,
  24), faces 1301 whole / 1302 burning from cicn 550 / 551, mask cicn
  598; `at` is not set by the setup): dir 1 = whole. Burning (dir 2–18)
  it flickers between its faces each frame and clears its level entry
  (it will not come back); at dir 19 it explodes: four `EasyExplosion`s
  at pos + (−20, −20), (−20, 0), (−20, −36), (0, −20) (h, v) with kind 0,
  frame 0 / −3 / −8 / −5 and its speed / 256 (toward 0), the view shakes
  32 frames (`_DAT_10065ad0`); next frame it shows the burning face and
  freezes (the switch falls through into case 30, so 21–29 never run).
  `timer` counts down each frame. Then `HandleGlide` / `HandleHalfPipe`
  (sound 0: none) / `JumpScale` (sound 800) with centre + 12. Tile task:
  `WallBounce` / `BGTileBounce` with cicn 598, centre + 12.
  Hit task (both on the floor): Harry's kick → 13 × kick thrust away from
  him, kick contact 128, then `SATRand(100)` < 50 and whole → it lights
  (3-D sound 801 at 62000 + `SATRand(6000)`), else sound 800 (if not
  playing). Harry walking in: pushed out (his pos and fix only, not his
  `at`), sound 800, the barrel takes 0.6 of his speed, he keeps −0.3; on
  caffeine it lights (sound 801). Another barrel: timer + 8 (past 25 it
  lights), both `pos` = `at`, the other pushed out, both × −0.4, then
  mine += other's, other's += my new. Cooler / plant / chair: pushed out
  (cicn 598 against 598 / 599), sound 800, a cooler bounces it back (×
  −0.4), plant and chair trade momentum (0.4, then × −0.4). Doors (not
  open), colliders: `RectBounce` (+ 12); ramps: `HandleRampCollision` (+
  12, sound 0). Shots (not the can 9610) stop on it, explosions push it
  (`HandleExplodeRadius` with + 18); either lights it, 9450 also pushes
  (speed / 4). Flames (effect kind ≥ 4200) set their +0x6e to 3 and light
  it. Desks: always a hit (no mask test, no jitter), sound 800, back ×
  −0.4 as from a cooler.
- **Chair** (560; layer 7, hot rect (8, 8, 28, 28), faces 1801–1836 (36
  directions, PICT 560), mask cicn 599): the setup takes a random
  direction (`SATRand(35)`); `f6e` = direction × 256, `timer` = spin.
  Each frame `f6e` += timer, timer × 0.9 (truncated), `f6e` wrapped into
  0…0x23ff, dir = `f6e` >> 8, face 1801 + dir; then glide / half pipe
  (sound 258) / jump (sound 257) with centre + 18 and the straight move.
  Tile task: the plant's. Hit task: explosions push it (+ 18). Harry's
  kick: 3-D sound 257, its speed is set to 13 × kick thrust away from him
  (kick contact 128), the spin −= turn × (new speed's `PEDistance` / 8),
  turn = direction from Harry's centre (at + 32) to his centre + the new
  speed minus that to the chair's centre (at + 16), the short way round,
  at most ±9; then the old speed is added back. Walking in: like the
  plant (Harry's `at` and fix moved, sound 258, 0.6 / −0.3), the spin +=
  turn (no limit) × (his new speed's `PEDistance` / 32). Cooler / barrel
  / chair: pushed out (599 against 598 / 599), sound 258, cooler × −0.4,
  barrel and chair trade momentum. Doors, colliders: `RectBounce` (+ 18);
  ramps carry it. Shots (all, even the can) stop on it: 3-D sound 258,
  9450 pushes. Plants are not handled here (the plant's hit task does it).
  Desks: always a hit, sound 258, back × −0.4.
- Both are blown by fans (`HandleRepulsions`) and bounce the vending
  machine's can; `HandleBarrelSprite` / `HandleChairSprite` clear a flag
  (`PTR_DAT_10065ee0` / `PTR_DAT_10065fb0`) nothing reads. Sounds: 800
  barrel bump, 801 barrel fire, 257 chair kick, 258 chair bump.
- **Desk** (301–319; `harry_core::desk`; direct, no level index; layer 1;
  `HandleDeskSprite` and `HitDeskSprite` are empty). Below 310 across:
  hot rect (0, 0, 32, 64), face entry kind − 301 of `InitFaces`' table at
  `_DAT_100658c4` (HrSp 6001–6005); 310 and up along: (0, 0, 64, 32),
  entry kind − 311 of `_DAT_100658c0` (HrSp 6011–6015). Only 6001 and
  6011 exist (64 × 32 / 32 × 64: a desk with a computer); the setup sets
  `at`, not `fix`. Harry's masks against them: HrSp 6021 / 6022
  (`InitFaces` with `BuildMaskRegion`, `_DAT_100658bc` / `_DAT_100658b8`).
  Episode 1 has no desk. Plants, barrels, chairs: always a hit, bounce
  back × −0.4 (their step back, `fix` − speed, is overwritten by the
  hit's `fix` from pos); robots and reversors: like a plant or cooler
  (move undone, turned). Harry: see "Harry touches a sprite". Recorded
  in `emu_playerhit.py` (2607 desk cases: side, corner, kick), 
  `emu_planthit.py`, `emu_furniture.py`, `emu_robot.py`, `emu_turret.py`
  (reversor), all matching; 9 mutations caught, one equivalent (the
  kick test at the desk's pos or at: always equal). Not reached:
  `GenerateSprite` for desks (no level has one; read from the code).
- `re/tools/emu_furniture.py`: 12 000 task runs of 1–8 frames on all
  levels (with tile collisions, explosions, level entries, the shake) and
  40 000 hit cases (Harry kicking / walking, barrel, chair, plant,
  cooler, doors, colliders, ramps, shots, explosions, flames made as
  explosions with a flame's kind), all matching; 13 of 14 mutations
  caught - the survivor is the chair's lower spin limit (the upper one is
  caught; a turn past −9 needs Harry almost on the chair).
- **Stairs** (9900–9909, idle at their position; layer 1, hot rect (0, 0,
  32, 32), no task or hit task of their own (both empty), faces 4501–4504
  for 9901–9904 from cicn 9901–9904; `SetupStairsSprite` also knows door
  kinds 922 / 923 (face of 9903) and 932 / 933 (9904), which never reach
  it). `HandleStairsCollision(sprite, half, hot, stairs)` - from Harry
  (on the floor, centre + 32, his normal hot rect; after the ramp test),
  plants and chairs (+ 18), barrels (+ 12): the sprite's centre against
  the stairs' `at`. 9901 leads down: moving down with the centre above
  at.v + 22 it starts down them once below at.v − 4 (`air` 200, speed h
  0, `flipKind` = the kind, which `JumpScale` then plays out); 9902 up
  (moving up, centre below at.v + 10, in once above at.v + 36); 9903 /
  9904 the same to the right / left (speed v 0). From anywhere else:
  `RectBounce` with the level's bounce. `re/tools/emu_stairs.py`: 30 000
  cases (all kinds, Harry and objects from all sides), all matching, 9
  of 9 mutations caught; stairs are also in the Harry, plant, barrel and
  chair hit recorders.
- **Buttons** (9501 / 9511 / 9521 / 9531 on walls, 9541 a floor plate;
  idle at their position; layer 1; hot rects (−8, 0, 8, 24) for 9501 /
  9511, (0, −8, 24, 8) for 9521 / 9531, (0, 0, 18, 18) for the plate;
  faces 1701… / 1711… / 1721… / 1731… four each, 1741–1745 the plate).
  The state lives in the level entry: info #1 = 1 while the button is up
  (dir < 7), 0 while down. `dir` = how far it is pressed (0–9): each
  frame it springs back by 1 unless it is at 8 or more and the entry's
  info #0 is 1 (it latches). On its first frame (+0x8b) a button whose
  entry has info #1 ≠ 0 starts at 9 (read without checking the index).
  Coming up through 5 (last frame's dir in `f6e` = 5, now less): release
  sound 4707 (plate 4705). Face dir / 2, the wall buttons' last face from
  8 on. `HitButtonSprite`: Harry not kicking, plants, barrels, chairs
  (robots, reversors, mail guys): dir + 2, sound 4706 (plate 4704) when
  it was up; shots: a wall button to 9 (sound 4706); Harry's kick rect on
  a wall button: dir + 10 (sound 4706).
- **Sliding doors** (950–999 idle, position even, 951 one pixel lower;
  951 across with hot rect (0, 0, 6, 32), faces 4401–4408 (PICT 851); 961
  upright (0, 0, 32, 6), faces 4451–4458; layer 5; no hit task of their
  own). With a level entry (index 0–255) each frame: the entry's info #0
  names the entry it follows - that entry's info #1 = 0 (its button down)
  opens it, a negative info #0 follows the opposite of entry −info #0
  (other indexes than 0–255 read past the list). Opening: dir + 1 up to
  14 (sound 4708 as it starts); closing: dir − 1 (sound 4709 as it
  leaves 14). Face dir / 2; from dir 8 on no hot rect. Harry (`RectBounce`
  with his hot rect, centre + 32, in the air too), plants, barrels and
  chairs bounce off it; player shots stop at it (sound 4700), enemy shots
  too (sound 4701, the wall guns' shot as well).
- `re/tools/emu_button.py`: 15 000 button runs of 1–20 frames with random
  partners hitting it in between, and 15 000 sliding door runs of 1–25
  frames with the linked entry changing, all matching; 12 of 12
  mutations caught (a 13th was equivalent). Sliding doors are also in the
  Harry, plant, barrel / chair and enemy shot recorders.
- **Co-workers** (1300–1449, idle with h even + 1 and v even; layer 5,
  hot rect (2, 2, 40, 40), no hit task). Faces from a table of 12 per
  group of 20 kinds: 1301–1320 the eleven frames 2101–2111 (cicn
  1301–1311), the seven groups from 1321 on only their first, 2302–2308
  (cicn 1321, 1341, …, 1441). Only 1301 moves: at dir 0 a 1-in-15 chance
  each frame (`SATRand(15)` = 1) to start, then dir + 1 a frame, holding
  at 12 (1 in 60 to go on), up to 32, then back to 0; face = dir / 3. A
  co-worker on layer 10 (just hit by a shot) goes back to 5. Harry (on the
  floor; `RectBounce` once a frame like colliders), plants, barrels and
  chairs bounce off them; a player shot costs 2 favor like a dart enemy.
  `re/tools/emu_coworker.py`: 15 000 runs of 1–60 frames, all matching, 6
  of 6 mutations caught; co-workers are also in the Harry, plant and
  barrel / chair recorders.
- **Mail guy** (1601 up, 1611 down, 1621 right, 1631 left; made directly
  with its level index; layer 9, faces 3701–3704 from PICT 1601–1631, hot
  rect = the face's bounds: (0, 0, 64, 34) upright, (0, 0, 34, 64)
  across). Its state lives in its level entry: on its first frame (+0x8b)
  `timer` = info #0, `dir` = info #1; every frame timer, dir and kind are
  written back (the index unchecked). `timer` counts toward 0 - from above
  reaching 0 it stops the cart (dir + 1). dir 0: running (only its own
  speed component set to ±15); else stopped: speed 0, dir + 1, and from
  41 on `SATRand(40)` = 1 turns it round - the opposite kind, dir 0,
  timer −9, the old way's speed set once more (it moves 15 px on before
  turning), `STPlayDynamicSound(400)` at its position with its speed
  (half volume when 1631 turns). It moves in whole pixels (`pos`, `at`).
  Tile task: running (timer and dir 0) into a wall shape that faces its
  way - 1601 {1, 5, 6, 8, 11, 13, 14, 18, 21, 26, 29}, 1611 {3, 4, 7, 9,
  10, 12, 15, 16, 23, 24, 31}, 1621 {2, 6, 7, 8, 9, 14, 15, 19, 22, 27,
  30}, 1631 {0, 4, 5, 10, 11, 12, 13, 17, 20, 25, 28} - timer 6. Hit task:
  shots stop (robots and blobs get +0x72 cleared). Harry touching a
  running cart (dir 0, `_DAT_10065e40` = 0): that cooldown 18, `Ouch`,
  comfort delta − 10, his speed + the cart's × 128. Plants, barrels,
  chairs meeting it (`mail_guy_crash`): both frozen, sounds 602 and 401,
  explosions at the cart (kind 1 frame 4 with its speed, kind 0 with half
  of it) and at the object − 20 (frame −4, its speed / 256), both level
  entries cleared, score + 5000. It presses buttons.
  `re/tools/emu_mailguy.py`: 6000 runs of 1–150 frames at the mail guys'
  places in Episode 1 with tile collisions, all matching, 8 of 8
  mutations caught; the crash is in the plant and barrel / chair
  recorders (with score and level entries now), Harry's in the touch
  recorder (with `_DAT_10065e40`); the dynamic sound with the cart's
  `pos` before the move (the recorder logs snd, volume, position and
  speed), moved as "Sound" below describes.
- **Robots** (1800–1849, 2000–2099, idle; `harry_core::robot`). Setup:
  facing Harry (`FindDesiredDirection`: the generic direction from pos +
  14 toward `gPlayerH/V` + 32), timer 25, layer 7, hot rect (4, 4, 24,
  24); `state` = hit points, `f7c` = mode. 1801: 3 points, mode turn;
  1802–1811 become 1811 (the turret, points kind − 1808, mode aim);
  1812–2000 become 1821 (points kind − 1814 = 7, mode turn). Big robots
  2001 / 2011 / 2021: 8 / 12 / 15 points (others 3), mode aim, hot rect
  (4, 4, 36, 36). Faces 4201 + dir (PICT 212), big 4701 + dir (+ 36
  while firing; PICT 2001). Task: timer − 1; clears the two robot wall
  flags (`PTR_DAT_1006619c` / `…98`) - the tile pass after all tasks lets
  only the first robot touching a wall / an obstacle bounce. Nothing
  happens 500 px or more from Harry (`RobotTooFar`, either axis) except
  half pipe and jump (centre + 12, sound 600). Small robots: no points
  → dying (level entry cleared, score + (kind − 1791) / 10 × 1250);
  modes: 1 turn (one step toward Harry a frame, the short way, then 2),
  2 drive (more than 3 off → 0; else accelerate by 20 in its direction
  while either speed is below 150), 0 brake (× 0.7 until both below 101,
  then 1), 4 the turret (faces Harry), 5 kicked (wait for the timer,
  then 0), 3 dead (frozen, explosion at pos − 20 with speed / 256, 3-D
  sound 602, Harry's chatter delay 20, the entry's info #3 made where it
  stood with `GenerateSprite`). Turning, braking, driving and the turret
  shoot now and then (`RandomRobotShot`: 1 in 60 for 1801, 1 in 35 else;
  a shot of the robot's kind from pos + (7, 8) with its speed + 1500 its
  way, sound 601). 1811 never moves; the others move by speed (24.8).
  Big robots: points 2000 / 2500 / 3000 (even out of reach); mode 4
  aims (`ApproachMode`, one step a frame), on target 6 fires for 25–94
  frames (`RandomMGShot` 2 in 5 a frame while more than 20 are left: a
  bolt from at + 16, three frames out along dir ± 2 - the wrap uses 35,
  so 36 and −1 stay and add no speed - flying with the robot's speed +
  1500 along dir, 1 in 10 one more, 1 in 5 of the rest one less; sound
  606 at 62000 + `SATRand(6000)`, the original only while fewer than two
  play; the robot shakes ±2 px). Hit task: barrels light (801) and it
  bounces (× −1.4); explosions (timer out) push it, layer 10, 3 in 10
  cost a point; ramps, stairs; plants, coolers, colliders, co-workers,
  chairs, doors (not open), sliding doors (desks, grates): move undone,
  × −1.4, driving → braking. Harry's kick: sound 600, a point, 13 × kick
  thrust, kick contact 128, mode 5 for 10 frames; walking into it: it
  bounces, on caffeine a point. Tile task: `WallBounce` (bounce 256, a
  bounce → braking) / `BGTileBounce` (254) with cicn 598. Shots
  (`HitPlayerShotSprite`; enemy shots if not big against big and past
  their first frames, `f7c` 0): layer 10, a point, `f6e` 0, sound 600,
  explosion of kind −1 at pos − 20 (player shots at big robots: − 12)
  while points are left. Harry bounces off them (half the level's
  bounce, on the floor, not kicking); plants take half their speed; mail
  guys take all their points; they press buttons; fans blow them.
  `re/tools/emu_robot.py`: 8000 runs of 1–80 frames at the robots' places
  in Episode 1 (Harry walking about, tile collisions, shots, deaths,
  drops) and 30 000 hits, all matching; 26 of 28 mutations caught (one
  acceleration constant and `ApproachMode`'s exact 18-step tie). Robots
  are also in the Harry, plant, enemy shot, button and fan recorders.
  (The recorders' `stub_routine` now replaces an earlier stub of the same
  routine; before, both ran and the first one's 0 reached the second.)
- **Fire pipes** (4201 up, 4202 down, 4203 right, 4204 left; idle,
  position even, face 6041–6044 (cicn 4201–4204) given to
  `AddIdleSprite`; other 4200–4299 make nothing). Setup: layer 0, an
  empty hit task, `f6e` = `SATRand(100)`. Task: `f6e` + 1; past 90 a 1 in
  40 chance each frame of a flame (`SATNewSprite(pipe kind, …,
  SetupEffectSprite)` at its mouth: 4201 at + (−3, −49) (h, v), 4202 (−4,
  16), 4203 (16, −2), 4204 (−49, −2); then kind = the level entry's info
  #1 - 0 in all of Episode 1 - and `f7c` = the pipe's kind), `f6e` 0,
  3-D sound 700. **Flames** (effects with `f7c` ≥ 4200): setup with the
  pipe's kind - layer 0, +0x8b set, hot rect along the flame (4201 (8, 3,
  64, 22), 4202 (0, 3, 56, 22), 4203 (3, 0, 22, 56), 4204 (3, 8, 22,
  64)); faces 2901… (PICT 214, up; 4202 + 16) and 2801… (PICT 216,
  right; 4204 + 16), frame dir / 2. Task: dir + 1; at 24 `f6e` + 1 and
  back to 7 while `f6e` < 3, else on to 25; gone after 28. Harry touching
  it (frames 7–24) sets its `f6e` to 3 (it dies down after this round).
  Explosion pushes skip flames (`f7c` ≥ 4200); barrels touching one light
  (as for any effect). `re/tools/emu_firepipe.py`: 15 000 pipe runs of
  1–300 frames and 15 000 flames of 1–80 frames, all matching, 12 of 12
  mutations caught (the pipe faces through the world recorder, which now
  has every ported kind).
- **Grates** (3900 across / 3905 upright break; 3910 / 3915 fences; idle,
  h even, v even − 1 (upright: v even); layer 8, hot rect (0, −8, 6, 72)
  across, (−8, 0, 72, 6) upright; faces 3401 / 3402 (cicn 3900 / 3905);
  `state` = hits left, 20 / 4). Out of hits a grate breaks: three
  explosions along it 29 px off (the last frame 4), sound 602, 600
  points, frozen. A fence goes out (sound 1200, `state` −100) and counts
  down: off from −102 on, the face shown on odd counts −129 … −101 and
  −229 … −201, back with 4 hits at −231 (about 4.3 s). A grate on layer
  9 goes back to 8. Hit task: explosions in their first frames (timer
  not 0) cost a hit (layer 10); Harry standing in a fence that is coming
  back (below −200) keeps it at −226 / −227 (`SATRand(2)`). Harry
  (`HitPlayerSprite`, fence not out): kept on his side by the grate's
  edge (his normal hot rect; which side from the grate's top / left + 3
  by `pos` against his centre), speed across × −bounce / 256, along ×
  bounce / 256; his kick rect on the grate's `hotRect2`: kick contact
  256, a hit, sound 4702. Plants, barrels, chairs: `GrateBounce`
  (`VBounce` across, `HBounce` upright: back on their side of the middle
  line, the speed across reversed and both × bounce / 256 if moving into
  it). Robots: like other obstacles. Player shots: a hit, layer 10,
  sound 4702 pitched 40000 + `SATRand(12000)` + (20 − hits) × 1800
  (fences (4 − hits) × 8000); enemy shots: a hit, sound 4702, layer 10;
  the can bounces off. The grate's draw proc puts a hit grate (layer 10,
  not a fence) to layer 9.
- **Ghosts** (5001: 3 hit points, slow; 5011: 5, twice as fast; idle at
  their position; layer 20, hot rect (0, 0, 40, 40), faces 3301 + dir
  (PICT 224); `at` not set by the setup). Each frame one step toward
  Harry (`FindDesiredGhostDirection` = the robots' direction,
  `ApproachMode`). Mode (`f6e`) 2 drifts at 384 / 768 along its
  direction (`AccelerateGhost`, cos table 1, .985, .94, .866, .766,
  .643, .5, .342, .174); mode 1 (hit) stands, timer 60 counting down,
  blinking (odd counts no face), at 30 it moves to a random place in the
  view (`SATRand(408)` both ways) whose 40 × 40 rect does not touch
  Harry's top-left − 32 … + 96, at 0 back to drifting; mode 3 (or hit
  points below 0): an explosion at − 12 (5001 kind 10, else 1; frame 4,
  timer 0), 2000 / 4000 points, gone. 1 in 1000 a frame sound 850. Only
  player shots hurt it (`HitGhostSprite`, timer 0): with points left one
  less, mode 1, timer 60, sound 852; else mode 3, the level entry
  cleared, 3000 points, sound 851; the shot is gone. Harry on the floor,
  not kicking, touching a drifting ghost: comfort delta − 7, damage
  cooldown 12, sound 224. Fans blow them.
- **Turrets** (1901 / 1911 / 1921 up, …2, …3, …4; idle; layer 7, hot
  rect (4, 4, 20, 20), timer 25, `f6e` 1; 25 / 40 / 50 hit points for
  190x / 191x / 192x; faces 4601… (PICT 1901, 32), none until the first
  frame). Each frame: timer − 1; Harry's direction (the robots') is taken
  when within the turret's arc (…1: 5–12, start 10; …2: 23–30, start 30;
  …3: 14–21, start 18; …4: below 4 or above 31, start 0); facing him it
  shoots (`RandomTurretShot`: 1 in 35 for 1910–1919, else 1 in 60; as the
  robots' shot, 1500 along its direction, sound 601). Out of hit points:
  frozen, explosion at pos − 20, sound 602, info #3 made, 1000 / 1500 /
  2000 points - its level entry stays, so it is back when Harry returns.
  Faces: …1 dir − 5, …2 dir − 15, …3 dir + 2, …4 dir + 28 (below 4) or
  dir − 8. Hit task: a player shot −5 (sound 600, the shot gone); an
  explosion (timer out) −1, layer 10 (the draw proc puts it back to 7).
- **Reversors** (1851 rolling right at 1000, 1852 down at 1000, 1861 /
  1862 at 1500; made directly with their level index; layer 7, hot rect
  (4, 4, 24, 24), the small robots' faces spinning, dir + 1 a frame).
  They move (24.8), ride half pipes and jump (centre + 12, sound 600) and
  bounce: walls (`WallBounce`, centre + 16, bounce 256, the first
  reversor of the frame only - flags `PTR_DAT_10066188` / `…84` as the
  robots'), obstacles. Hit task: barrels light (sound 801; the barrel's
  dir + 1 even when burning) and it turns back; ramps, stairs; plants,
  coolers, grates, chairs, doors (not open), reversors, co-workers,
  colliders, sliding doors (desks): move undone, turned back. Shots are
  sent back (their move undone, speed reversed, sound 605) and the
  reversor turns a quarter (its larger speed component moves to the other
  axis). Harry on the floor touching one: comfort delta − 8, `Ouch`,
  damage cooldown 12 (in the air too: no). Buttons, fans.
- **Boulders** (6001 up, 6002 down, 6003 left, 6004 right; 6011–6014 12
  px a frame instead of 8; made directly; layer 200, hot rect (16, 16,
  48, 48), faces 1601… (PICT 6001, 12, turning with `f6e` 0–5)). Like the
  mail cart: state in the level entry (timer info #0, dir info #1, the
  start coordinate info #2 - written on the first frame if 0, else
  taken), the wall list per way the mail cart's, timer 12 at a wall;
  stopped, from dir 4 on a 1 in 5 chance a frame to restart from its
  start (sound 402 at 62000 + `SATRand(6000)`). Hit task: a robot it
  touches loses all hit points (blobs too). Harry on the floor: his kick
  rect on it gives kick contact 256; otherwise, when not already
  shaking (`_DAT_10065e68` = 0), he is run over: shaking 32 frames,
  sound 224, comfort delta − 18, standing.
- `re/tools/emu_turret.py`: 6000 runs each of turrets, reversors and
  boulders (1–150 frames at their places in Episode 1, Harry walking,
  tile collisions, partners), all matching; 18 of 19 mutations caught
  (the survivor is equivalent). They are also in the Harry, button, fan
  and world recorders (the touch recorder now has the shake counter).
- **Scientists** (1111 / 1211 right, 1121 / 1221 left, 1131 / 1231 up,
  1141 / 1241 down; idle; layer 9, hot rect (8, 8, 24, 24); faces 4301 +
  pose + 8 × group (PICT 1111)). Only while `at` is within the view − 64
  … + 512. `f6e` counts a throw: 0 waits (pose 1) until Harry's direction
  (the robots') is in its arc (1111: below 5 or above 30; 1121: 15–21;
  1131: 6–12; 1141: 24–30) and keeps it in `speed.h`; 1–8 wind up (poses
  1–3); 9 throws (pose 4, 3-D sound 300): a shot of its own kind from
  its hand (1131 pos + (20, 2) (h, v), 1121 (4, 4), 1111 (22, 20), 1141
  (5, 19)), 1200 along the kept direction (12xx 1800; the unit table of
  the ghosts), then timer 0 and dir 0 - the 11xx / 12xx shots are the
  homing ones, so it homes at once; 10–20 follow through (poses 4–7);
  then, once the timer (a hit's pause) is out, it counts on and starts
  over at random (11xx 1 in 25 a frame from 46, 12xx 1 in 15 from 16).
  Layer 10 goes back to 7. Hit task: a player shot is gone, sound 4700,
  layer 10, corporate favor − 3, a pause of 65–134 frames (the shot's own
  hit task takes 2 more and may make it say something). Harry bounces
  off it like off a co-worker.
- **Blobs** (1701 5 hit points, hops at random with 3200; 1711 7 points,
  lunges with 4500, 2 in 3 times at Harry; 1721 5 points, creeps after
  him 350 a frame for 15 frames, 93 in 100 times at him; idle; layer 7,
  hot rect (4, 4, 24, 24); faces 1401… (PICT 1701: 16 turning, 8 dying);
  setup: a random direction 0–34 and wait 0–89, resting). Modes (`f6e`):
  2 set off (`AccelerateBlob`, the unit table), 0 slide (× 0.7 until both
  below 101, then rest for 30–89 / 15–54 / 5–64 frames; 1711 and 1721
  set off again at random, 1 in 20 / 1 in 34 a frame), 4 rest (the timer
  counts down, turning a step a frame below 40 and another below 20, then
  set off). At most 2000 per axis (`EnforceMaxSpeed`); face dir / 2 of
  16; half pipes (centre + 16, sound 604) and, in the air, jumps. Out of
  hit points: 32 frames of dying (faces 1416…), then 500 / 1200 / 2500
  points and info #3 made. Hit task: explosions push (+ 16) and cost a
  point (layer 10); doors (open ones too), plants, barrels, coolers,
  grates, boulders, sliding doors, chairs, co-workers, colliders: move
  undone, × −1.1, sliding; Harry's kick: sound 600, a point, a quarter of
  13 × kick thrust ((kick × 26) >> 2), kick contact 64. Tile task: walls
  (`WallBounce`, centre + 16, bounce 255, cicn 598) and obstacles (it
  bounces back) - the first blob of the frame only (flags `…f00` /
  `…efc`). Player shots: with points left one less, layer 10, `f6e` 0,
  sound 604 (and 603 for the last). Harry on the floor touching one:
  sound 224 and comfort delta − 6 (1711 and up − 9), damage cooldown 6
  (12). Mail carts and boulders finish them; fans blow them; the draw
  proc puts a hit blob back to layer 7 (+0x8a).
- `re/tools/emu_blob.py`: 8000 scientist runs (view and Harry moving,
  shots) and 8000 blob runs (tile collisions, partners) of 1–150 frames
  at their places in Episode 1, all matching; 28 of 28 mutations caught.
  `re/tools/emu_playershot.py`: 30 000 player shot hits on every kind its
  hit task knows (colliders, sliding doors, dart enemies, co-workers,
  scientists, robots, blobs, grates) and some it passes, all matching -
  except the blobs' setup face past its table (BUGS.md), which the tests
  leave out. Scientists and blobs are also in the Harry, fan and world
  recorders.
- **The boss** (2201, level 15; idle; layer 7, hot rect (8, 8, 56, 56),
  150 hit points, faces 4902 + dir (PICT 2201); its turning state in
  globals, `harry_core::boss::BossState`: phase `_DAT_1006c350`, pulse
  direction `DAT_1006c352`, phase change counter `…f80`, anim `…f7c`,
  sweep count `…f5c`, spin `…f58`, angle `…f54`, shot cooldown `…f3c`,
  explosion cooldown `…f44`; the setup resets count, spin, angle, change
  and sets phase 1 - not the cooldowns, the anim or the pulse). Each
  frame: half pipes and jumps (centre + 16, no sound); `f6e` counts up to
  (hit points + 2) mod 51 for the flash; no speed of its own; the
  explosion cooldown counts down; below 0 hit points it starts dying.
  Phase: 3 below 50, 2 below 100, else 1; a change spins it in place for
  100 frames (the view shaking 16/17, a roar 4901 at 32000 two frames
  in). A sweep: at its start a coin turns the spin by ±1, then it builds
  up by 12 a frame to ±600 within 60 frames and settles back to ±256
  after 100; while beyond ±200 it fires when the cooldown is out - phase
  1 the shot 2281 (cooldown 6–10), phase 2 the homing 1211 (5 frames out
  to the side first, `f7c` 1, timer and dir 0; cooldown 30–54), phase 3 a
  ghost 5011 at at + 12 with its direction and no hit points (cooldown
  30–54) - sound 300 at a random pitch. After 120 frames, facing Harry,
  it stops (count −50) and fires volleys of three 2291 shots at −45,
  −30, −15 (phase 2: −45, −35, −25, −15; phase 3 five, twice as fast;
  sound 1002); phases 2 and 3 face Harry while the count is below 0.
  Phases 2 and 3 pulse in size (± 4 between 1.0 and 1.19, ± 12 up to
  1.75) unless it is more than 200 px left of the view's middle. The
  direction comes from the angle (+ spin, 0 – 0x23ff). Hit task: an
  explosion −6 (then 30 frames not again); a player shot −1, the flash,
  every 8th point a cry (1000 / 1001, the original only when neither
  plays); Harry's kick −1, sound 600, layer 10, kick contact 64. Harry
  bounces off it like off a robot. Death (`HandleBossSpriteDeath`): it
  grows to 1.5 and spins at 800; 10 000 points, the ghosts lose their
  hit points, the music fades, no more hits, roars at 0 / 8 / 16 (34000,
  40000, 24000); twelve explosions about it from frame 50 to 96 (sound
  602 at 50000, stopped and restarted); at 114 two explosions on layer
  500 at twice the size, three more, the view shaking 48; gone after
  120. `re/tools/emu_boss.py`: 4000 runs of 1–400 frames (hit points,
  phases and globals at random, Harry and the view moving, hits), all
  matching; 26 of 29 mutations caught (one equivalent, two needing rare
  combinations of hit points and frames).
- **Drawing changes sprites.** `DrawSpritesBehindTiles` /
  `DrawSpritesAboveTiles` (after `EraseSprites`) call `WrapDrawSprite`
  for every sprite with a task and a face whose `at` lies within the
  view − 64 … + 512; the face's draw proc runs when the face reaches into
  the 448 × 448 view. Some draw procs change the sprite: robots, big
  robots and turrets on layer 10 (hit) go back to 7, blobs too (and get
  +0x8a), a hit grate goes to 9, the vending machine sets its +0x78. The
  port has the draw pass in `Game::frame` (`draw_pass`): two passes
  (+0x8b set: behind the tiles, then above), the face's size - or for a
  scaled sprite the position and size from `FindScaledPosAndBounds` -
  wrapped on the 480-px ring buffer (`wrap_draws`) around the scroll
  origin (`FUN_100251c8`: clamped to ≥ 0, 3648 past tile column 128)
  gives how many times the draw proc runs, and each run has its effect
  (`drawn`): Harry's `TintDrawProc` (faces 1101–1244) sets +0x8a (kind
  10; or, off caffeine, unless he holds the fire extinguisher 11999 with
  fire held, only when comfort fell below the shown value) or, on
  caffeine, draws `SATRand(10)`; robots, big robots and turrets 10 → 7;
  blobs and grates mark themselves drawn; the vending machine (1902–1903)
  sets +0x8a if +0x78 was 1 and its entry's info #1 < 1, then +0x78 = 1;
  the boss (4902–4937) sets +0x8a while `f6e` < 0, else in phase 3
  (unless animation 2 or 6 in phase 2) draws `SATRand(15)`. The draw
  procs are called through `__ptr_glue` (0x1006536C, the TVector in r12)
  with no null check: the recorder gives faces without one
  `BlitEncFaceProc`, as the face loaders do. `DrawSpritesBehindTiles`
  first draws the background props (table 0x100723ac), then the sprites
  with +0x8b, then the foreground props (0x100720ac), each prop through
  its face's draw proc without a sprite (the props' faces have
  `UserTintableSpriteDrawProc`: the level's tint). The port's draw pass
  makes the same calls (`tint::draw_proc`, see "Sprite tints"); the gun's
  proc also puts a B.A.D.A.S.S. fire hold over 66 back to 61.
- **Mirrored masks.** `BuildMaskRegion` on an encoded face mirrors the
  bits within each byte, so the wall masks that robots and reversors
  (face 4251, cicn 598 encoded) and blobs (face 1451) test against are
  mirrored too (`masks::from_face`); the cicn itself is not.
- `re/tools/emu_grate.py`: 15 000 grate runs of 1–300 frames with hits,
  30 000 `GrateBounce` cases and 15 000 ghost runs of 1–120 frames (Harry
  and the view moving, shots), all matching; 18 of 21 mutations caught
  (the survivors are equivalent or need an `at` that does not move).
  Grates and ghosts are also in the Harry, plant, barrel / chair, robot,
  enemy shot, fan and world recorders. (The grate hit sound is 4702 -
  `_DAT_10065ba8`; two of the touch recorder's sounds were not mapped.)
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
- **Sentences** (`GenerateBuzzwords`, `GeneratePhonewords`): a template
  from `STR#` 6900 (buzzwords; fillers 6901–6905 for `^0` … `^4`) or 6800
  (phone; names 6801 for `^0`); for each placeholder in turn a filler is
  picked and replaces its next occurrence (`Munger`) until none is left -
  one pick more than occurrences. `PickAString`: QuickDraw `Random` as
  unsigned, modulo the count, again while it equals the list's last pick
  (globals 0x10066e6c … 0x10066e7a, 0 at the start, kept for the whole
  run). A lowercase first letter (or second after `"` / `“`) is
  uppercased; the result keeps its length mod 256. They draw random
  numbers, so they are game logic (`harry_core::words`):
  `re/tools/emu_words.py` 5000 runs, the message, collider and whole-frame
  recordings now use them (the harness gained a small Memory Manager:
  `PtrToHand`, `Munger`, `GetHandleSize`, `BlockMoveData`).
- **`ShowBalloon`**, the game's part: shown with mode 1, 3 or 5 →
  `SATRand(16000)` (the sound's pitch); not shown with prefs +8 = 3 and no
  page scroll running → the view moves 16 px toward the sprite (h and v
  each by the sprite's `pos` + 32 against Harry's + 32). `harry_core::game`
  does both after the frontend's answer.
- **`ShowBalloon`** (frontend part): TextEdit layout in
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
    2500, comfort − 5 unless on caffeine. The counter is 2 at launch (the
    data section) and never reset: it carries over between levels, and
    each magnet in reach counts it down.
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
- `re/tools/emu_game.py`: 900 runs of up to 300 frames on level 1 with
  Harry's keys held at random - moves, and in most runs an inventory of
  guns, the power of the swivel, soda cans, caffeine and the mystery vial
  with use / select / 1–4 (start or next to a random sprite; the globals
  restored to the post-init data section per run; a run ends when Harry
  dies). All 892 comparable runs match - list, free list, every record,
  Harry's path, view, meters, inventory, ammo, random seed. The other 8
  kick the locked door 913, which reads stale stack values (BUGS.md). Not
  covered yet: Harry's death itself, the ripple, the frontend (since
  covered except the frontend).
  Now 1500 runs over all 15 levels (level 1 a little more often) with
  the real draw pass (`DrawSpritesBehindTiles` / `…AboveTiles` with the
  draw procs, the scroll origin each frame): all 1494 comparable runs
  match. `SEED=<seed>` reruns one case with per-frame dumps, `TRACE` /
  `RAND` print routine calls and the callers of `Random`; the test's
  `GAME_DUMP=<seed>` prints the same for the port.
- **Harry's death** (`harry_core::death`): `HandlePlayerSprite` hands
  Harry to `HandlePlayerSpriteDeath` (timer 0) once comfort or favor
  *shown* is below 1 and he is on the ground. Each frame clears the shot
  kind and the recoil. Frame 0: `FadeMusic(0, 1)`, no hit / tile hit
  task, no current item. He turns 2 steps a frame (frames 1–18), 3
  (19–30), 4 (31–45), 5 (46–51); frame 18 plays 4901 (comfort gone) or
  4900 (favor gone); frame 45: an explosion on him (`dir` 4), four
  flames 4204 / 4203 / 4201 / 4202 left, right, above and below his
  centre with `f6e` 3 (their last round), `f7c` their kind and +0x8b,
  four explosions ±16 about him with +0x8b, the burst 602, a shake of
  36. His face is the standing one (1101 + `dir`) until frame 51, then
  none. Frame 110 freezes him (so `EraseSprites` takes him out) and sets
  the "player off" flag `_DAT_10065acc`: `CheckGameEvents` then ends
  `GameLoop` (`DAT_10068560`, cleared before the loop), `HandleSprites`
  skips `CheckPlayerCollision`. After the loop: `EndLevelEffect`, then
  `AskToContinue` (not ported; the port starts the level again).
  `re/tools/emu_death.py`: 3000 runs of 1–130 frames from any timer,
  all matching; 8 of 8 mutations caught. The whole-frame recorder now
  runs on to the loop's end; 72 of its runs play the whole death.
- **`UpLevel`** (`Game::up_level`): `CheckGameEvents` calls it at the
  end of a frame in which the level counter changed; the game loop goes
  on in the next level. It plays 4800 (`_DAT_10065b54`, volume 10),
  shows `EndLevelEffect`, removes the debug items 11397–11399 (select,
  use up, until none is left), and ends the loop with `Win` when the
  counter is past the world's levels (Hwld +0x1c6: 15 in Episode 1).
  Otherwise the outro picture (level +0x1207e); counter −255 means the
  level after the one before (`_DAT_10068568` + 1). An unregistered
  copy stops after level 5 (`FUN_10057cb0` → `LevelLimit`; the port
  plays as registered). Then the score grows by (level − 1) × 5000, the
  state becomes the level's start (comfort, surface, favor, score,
  ammunition, the inventory copy - what `DoSaveGame` saves; autosave if
  prefs +1), `KillAllSprites`, `OpenDefaultWorldLevel`, `IntroLevel`,
  `0x1007e3a6` = 69 (never read), Harry's position = start − 32, view
  and target = start − 240, the intro picture, `SetupLevelSprites` (it
  loads tile sets, tile kinds, physics and sounds), Harry at start − 32
  with the start direction, his weapon at the start (it moves onto him
  in its first frame; `SetupHeldWeaponSprite` sets only its `pos`, so
  until that frame its `at` is whatever the reused record held - a run
  that ends right after a level change showed it), the music. Unlike `GameLoop`'s start there is no
  `FindUpperLeftCorner` and the generated sprites see the new position.
  `InitGameGlobals` (a new game) sets the level start to 72 / 72 / 72
  and nothing else.
- The whole-frame recorder lets `UpLevel` run (pictures, dialogs, saves
  and music stubbed; the registration check says registered; the next
  level's data put where `OpenDefaultWorldLevel` would) with the world
  loaded, starts some runs next to an exit and compares score and level
  start too: 37 runs change level (and play on), 10 win.
- **Track map** (`harry_core::track`, for the map screen `ShowMap`):
  128 × 128 bytes at 0x10072730, `[h * 128 + v]` in 32-px tiles.
  `UpdateTrackMap`, the last of a loop pass: when the tile under Harry's
  centre (`gPlayerH/V` + 32) changed, the tiles from 7 before to 6 after
  it (14 × 14) are seen; the last tile (`sRam1007272e` / `…2c`) is never
  reset. `SetupLevelSprites` clears the map (`InitTrackMap`) unless a
  restored copier save brought one (`uRam10072719`). Whole frames
  compare the seen tiles and the last tile: all matching.
- **Saved games** (`harry_core::savegame`, `Game::do_save_game`,
  `copier_save_game`, `continue_game`): one record of 0x95d8 bytes
  (`_DAT_10065b60`) for the whole session, written by `SaveSG` as
  resource `HrSG` 0 of a file of type `HrSG`, creator `Hary` (layout in
  the module). Saves change only their part of it:
  - `UpLevel` clears the copier flag (+4) and, with prefs +1 (autosave),
    calls `DoSaveGame`: world id, level, the level-start meters, score,
    ammunition and inventory copy, the date, the world file's name -
    not position, track map or sprite list.
  - `CopierSave` (dialog: Save / Cancel) → `DoCopierSaveGame`: world id,
    copier flag, level, the current meters, score, ammunition,
    inventory, `gPlayerH/V` and Harry's direction, track map, the
    level's sprite list (positions − 6) and custom tables - not the
    world's name - and all of it becomes the level's start.
  - `ContinueGame` (from the menu, or after a death when `AskToContinue`
    says so - it asks only once something was saved or opened this
    session, `PTR_DAT_10065954`, and prefs +0x22 is 0; then it reads that
    file): `InitGameGlobals`, `OpenSG`, the record's values into the
    globals and the level start, `PTR_DAT_10065904` set (the next
    `SetupPlayerSprite` rebuilds the inventory with `GetInvItem`, so the
    first item is selected), the level; a copier save also brings its
    track map (kept by `SetupLevelSprites`), sprite list and custom
    tables and starts `GameLoop` at the saved position and direction
    (with `gPlayerH/V` 32 less until the first frame), otherwise at the
    level's start.
  The whole-frame recorder runs the real `CopierSave` (dialog answers
  per run), `DoSaveGame` (autosave per run) and `ContinueGame` (15 % of
  the runs start from a random record: copier or not, other meters,
  score, track map, sprites gone; `GameLoop` reports where it would
  start) and compares the record: all matching; 8 of 9 mutations caught
  (the survivor needs chatter, which the runs leave at 0). The frontend
  saves at every level start and at every copier (no dialogs yet) to
  `Harry Save.hrsg` in SDL's pref path and continues from it after a
  death.
- **Ripple** (`harry_core::ripple`): level info #10–#12 are its length
  (rows per wave), width (px) and speed (1/256 rows a frame); in Episode
  1 only level 10 ripples (40, 3, 384); W-A-V-Y-G-R-A-V-Y sets 100, 16,
  0x300 or 0, 0, 0 and the phase to 0. The phase (`*_DAT_1006592c`)
  grows by the speed after the drawing of every loop pass and is never
  reset otherwise (it carries over levels). `BuildSineTable` (from
  `IntroLevel`, so at every level start, and from the cheat) cuts the
  length to 100 in the level itself and fills entry i with
  `(int)((float)sin((float)((float)(6.28318 / length) · i)) · width)` -
  the product in single precision (`emu_sine.py`: 7865 lengths and
  widths, all matching; in double precision six differ by 1). 6.28318
  is the original's 2π. `DoubleBlitUniversal` uses `DoubleBlitSine`
  when the width is at least 1, the frame is not a full redraw and the
  scroll mode is at most 2: screen row y of the game area is shifted by
  table[(view v + phase / 256 + y) mod length] - right leaving its first
  pixels as the screen had them, left reading past the area's right
  edge in the ring buffer. The port's renderer draws the area 480 px
  wide and copies it row by row the same way (the left pixels of a row
  shifted right: unshifted). The whole-frame recorder types the cheat in
  6 % of its runs (the phase, as `GameLoop` adds it, and the table
  compared): all matching.
- **Shots made with their own fix**: the boss's (2281, 2291, 1211) keep
  `pos` / `at` where `SATNewSprite` put them until their first frame -
  so their hot rect is there for that frame's collisions - while the
  robots set `pos` and `at` from `fix` right away (`to_fix`).
- **Player shots in the frame**: records with `SetupPlayerShotSprite`
  (layer 11, timer 3, hot rect (−2, −2, 10, 10), the swivel (10) (27, 27,
  37, 37), +0x8b); faces staple 4001 + frame, B.A.D.A.S.S. 4011 + frame,
  soda can 3061 + frame (the can's set), swivel Harry's 1101 + dir; the
  held weapon's faces 3501 + dir, recoiling 3551 + dir (the gun is only a
  tint), at the level start 3501 + `gPlayerDir` whatever he holds.
- **`HitPlayerShotSprite`**: colliders and sliding doors stop the shot
  (3-D sound 4700); dart enemies, co-workers and scientists stop it too,
  cost 2 corporate favor and half of the time say one of 253–256; the hit
  one gets layer 10 and +0x8a. A soda can (kind 4) bursts on anything
  except Harry, his weapon and effects: sound 500 and `EasyExplosion` at
  its position − 28, kind 14. (Robots, blobs and grates: with their
  kinds.)
- **Explosions** (`EasyExplosion(h, v, speed h, speed v, kind, dir)` =
  `SATNewSprite(kind, …, SetupEffectSprite)`, then speed, dir and kind):
  layer 20, no hit task, hot rect (4, 4, 60, 60) for kinds ≥ 0 (none for
  −1, the robots' and the mystery vial's); 13 faces 2701…, frame `dir` /
  2, gone after frame 26; they drift by their speed in whole pixels.
  `HandleExplodeRadius(sprite, half, effect)`: once per 30 frames
  (+0x96) a sprite within 32 px (`PEDistance` = larger + smaller / 3 of
  the centre distances; the effect's centre at + 32) is pushed away by
  (32 − d, at most 16) × 128. Plants (half 18; only by explosions with a
  hot rect) also lose a state and go to layer 10; Harry (half 32, on the
  floor) in the explosion's frames 6–19 loses 8 comfort (ouch, cooldown
  15). The fire pipes' flames (`f7c` ≥ 4200) come with the fire pipes.
- **Harry's death**: `HandlePlayerSprite` sets the task to
  `HandlePlayerSpriteDeath` and the timer to 0 when a shown meter is
  below 1 on the floor; from then on `CheckPlayerCollision` finds no
  player. The weapon, the enemy shots and the colliders find Harry
  through `*_DAT_10065980`, not his task.
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
- **Desk** (also in the air): Harry's SAT position from his `fix`, then
  the mask test (cicn 299 against HrSp 6021 below kind 310, else 6022)
  moves him out. No hit: kicking with his kick rect (at pos) on the
  desk's hot rect gives kick contact 256. A hit: from his centre (pos +
  32) against the desk's rect (its pos; width 64 below 310 else 32,
  height 64 above 310 else 32 - kind 310 is 32 × 32): within its width
  speed h × 0.5, v × −0.5; within its height h × −0.5, v × 0.5;
  otherwise (a corner) both × −0.5; `fix` from pos. (The constants are
  at the Ghidra address + 4: `_DAT_10066848` reads −0.5 from 0x1006684c.)
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

### Sprite tints (draw procs, `BlitEncFace` modes)

- Every face has a draw proc (face +0x18; `BlitEncFaceProc` unless
  `InitFaces` or a sprite's init routine sets another). It calls
  `BlitEncFace(face, port, …, mode, arg)`: mode 0 plain, 1 a tint table,
  2 translucent, 3 reddening, 4 heating; mode 1 with arg −1 becomes 2.
  `arg` is the table index; a scaled sprite adds its scale << 5 (from
  0x20 on the face is drawn scaled, the index in the low 5 bits, and
  untinted when that is 0). `NoScaleDrawProc` and some procs pass no
  scale (drawn unscaled).
- Tables (`BuildTintTable` at launch, from the screen device's colour
  table, each RGB mapped back with `Color2Index`):
  - `_DAT_10065d00` 16 tints, entries 0–254 (255 black stays; 13, 14 all
    256): 0 identity, 1 red (grey + 10000), 2 yellow, 3 blue, 4 (0, grey,
    FFFF), 5 light grey (+ 16000), 6 half, 7 quarter, 8 × 1.5, 9 × 2.5,
    10 random (`SATRand(5000)` for blue, then red; green the grey), 11
    grey, 12 (FFFF, grey, 0), 13 green and blue halfway to FFFF, 14 brown,
    15 identity.
  - `_DAT_10065d10` translucent: by s = 0–15 (the face pixel's grey >> 12,
    `_DAT_10065d14`) the pixel below lightened by s << 11, then grey.
  - `_DAT_10065d0c` redden, 8 steps: toward ((grey + FFFF) / 2, 0, 0) by
    (s + 1) / 8; `_DAT_10065d08` heat, 16 steps toward (g, g, g / 2) with g
    = (grey + FFFF) / 2 by (s + 1) / 16. Black stays.
  - `re/tools/emu_tint.py` runs it with `Color2Index` logging: all 13 812
    colours and the 57 tables match the port (`tint::Tables`).
- `Color2Index` uses the device's 4-bit inverse table (`MakeITable`). The
  port takes the nearest colour of clut 8 for each cell (`tint::
  inverse_table`), an approximation: the ROM's construction is not
  reproduced.
- The procs (port: `tint::draw_proc`, by face range `tint::proc_of`):
  - Harry (`TintDrawProc`): kind 10 translucent; on caffeine tint
    `SATRand(10)` + 1; spraying the fire extinguisher heat fire hold / 2
    (above 15 cycling 15, 14, 13, 12, 13, 14); comfort falling redden
    (shown − comfort, at most 8) − 1; else the level's tint.
  - The level's tint is level info #0 (Hlvl 0x12074); most procs use it
    when it is not 0. `UserTintableSpriteDrawProc`: else the level
    entry's info #0 (−1 … 15, else 0); buttons and the mail guy: info #3
    (with a level tint, level info #3); messages: info #2.
  - Hit (layer 10): robots, turrets, reversors tint 4 (and back to layer
    7), blobs 8 (+0x8a, layer 7), grates 4 (layer 9), scientists and dart
    enemies 1.
  - By kind: robots (1801 plain, 1811 2, 1851–1852 4, 1861–1862 10, other
    1812–1899 1, the rest not drawn), reversors (2011 1, 2021 4, else 0 -
    they ignore the level's tint), turrets (< 1910 plain, < 1920 8, else
    12), scientists (1200–1300 6), ghosts (5001 plain, else 1), fences 1,
    blobs (1701 plain, 1711 10, 1712–1799 1, the rest not drawn), effects
    (kind −1 translucent, 5 → 4, else their kind; plain for 0 or f7c >
    4199), doors 903 / 913 / 923 / 933 (by entry info #0: 0 or no entry
    4, 1 2, 2 10, else not drawn), copier (9200 plain, else 6), vending
    machine (6 when empty and drawn before).
  - The held gun (`WeaponTintDrawProc`, Harry's scale): staple gun plain,
    shrapnel gun 4, B.A.D.A.S.S. 12 / 8 blinking by the fire hold from 7
    on; above 66 the proc puts the fire hold back to 61.
  - The boss: hurt (`f6e` < 0) reddened 7 at a phase change, else
    (0.16 · (50 − state mod 50), 2 … 8) − 1; in phase 3 a random tint
    (`SATRand(15)`); in phase 2 by its animation 0 → 7, 1 / 7 → 6, 3 / 5
    → 8, 4 → 9.
  - The fans (`ACDrawProc`) and dart enemies also draw a second face
    (`tint::second_face`): fans 2301 / 2302 / 2303 / 2304 an 8 px grille
    band (faces 1931–1934, cicn 2305–2308) over their top / bottom (row
    24) / left / right (column 24), plain; a dart enemy whose +0x66 is not
    0 and whose entry has info #0 = 1 its loaded dart, face 3041 (the
    first enemy dart face), at `GetDartEnemyProjOffset` (by kind 1011 /
    1021 / 1031 / 1041 and +0x66 = 1–6) in the level's tint. The original
    clips both to the part of the sprite being drawn (the dart's width
    computed wrongly at the right edge); the port draws them whole. Not
    in the whole-frame comparison (the recorder leaves second faces out).
- The whole-frame recorder logs every blit of the draw procs (face, mode,
  arg) and compares them with the port's (`Game::blits`); all 1500 runs
  match. Planted bugs: 5 of 8 caught; not reached by the runs: a fire hold
  over 66 and the kind boundaries 1920 (turret) and 1200 (scientist).

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

- **The mixer** (in the application, `_DAT_10066600`: up to 16 voices of
  0x34 bytes, `_DAT_10066f18` of them in use, sorted by priority
  (+0x30) and loudness (+0x2c + 0x2e)). `FUN_1005692c` (behind
  `STPlayRegSound`, `STPlay3DSound(Pitched)`, `STPlayDynamicSound`)
  inserts a voice where it belongs; with 16 and none quieter it plays
  nothing and returns 0. A voice's handle (+0) is the counter
  `_DAT_10066f00`, + 2 each time; +4 is the `snd`. `STPlayRegSound`
  returns −1 with sound off. `FUN_10056de8(x)` counts the voices whose
  handle or `snd` is x - **x = 0 counts every voice**; `FUN_10056c0c(x)`
  stops them the same way (0: all); `FUN_10056f94(x, params)` sets a
  voice's priority, volumes and rate. `HaltTalkingSounds` stops 13 `snd`s
  (the voices and the chatter).
- **Who asks** (`FUN_10056de8`): `HandleKeys` (a voice or the chatter:
  `gVoiceChannel`, `_DAT_10068aa4`; the gulp `snd`), `HitPlayerSprite`
  and `Ouch` (the voice), `HandlePlayerSprite` (the chatter: a playing
  voice is stopped first, then a new chatter only if the last one is
  over - and only then `SATRand(3)`), the plant's, barrel's and chair's
  bump (1100 / 800 / 258), `HitPlayerTileSprite` (the water sound),
  `HandleColliderSprite` (the fan), `RandomMGShot` (606: fewer than two,
  then `SATRand(6000)` for the pitch), `HitBossSprite` (1000, 1001:
  neither, then `SATRand(100)`), `UpdateDynamicSounds`. The voice and
  chatter handles are 0 until the first one: until Harry first speaks,
  "is a voice playing" asks about every sound (BUGS.md).
- **Dynamic sounds** (`STPlayDynamicSound(snd, priority, volume, at,
  speed)`, only the mail guy's 400): the first free of 16 slots
  (`_DAT_10065d34`, 0x18 bytes: used, handle, snd, priority, volume, at
  (v, h), speed (v, h)) takes it; none free, it is not played.
  `UpdateDynamicSounds` (each pass after `CheckGameLoopKeys`), slots
  0–14: a slot whose sound is over is freed, the others move `at` by
  `speed` and get new ear volumes from `CalcStereoVolume`
  (`FUN_10056f94`). `InitDynamicSounds` (at launch) clears 0–14.
- **The port** (`harry_core::sound`): every sound gets a handle; the
  frontend reports before each frame what still plays (`Playing`), and
  the game counts what it starts and stops within the frame as the
  mixer would. All the questions above are answered from it, with the
  original's handles (0 at first). `Audio` keeps the dynamic slots and
  sends `Pan` for each move. Differences: the frontend mixes 8 sounds and
  drops the oldest effect, not the original's priority list of 16 (a
  sound it drops counts as playing until the next frame; the game never
  gets a 0 handle from a full mixer); a voice that `HandleKeys` starts in
  the same frame as the chatter is not stopped by it. Tests: unit tests
  for the counting, the slots and the moves; the recorders already vary
  "playing" for the keys, the fan and the bumps; the mail guy recorder
  logs the dynamic sound's position; the whole-frame test runs with
  `Audio::silent` (its recorder stubs the mixer: nothing plays).

### Music `Tune` (Harry Music 600–620)

17 tunes (QuickTime Music Architecture): 600 intro / main menu, 601 levels
1 and 2, 603–615 one per level (level +0x120fa), 616 the end (Hwld +0x14c),
620 the story (Hwld +0x144, `ScrollPlot` with music). 32–91 s each.

- **Playing** (`SetMusic(id)`): the resource split at its first long
  (the header's size): `TuneSetHeaderWithSize(header)`, then
  `TuneQueue(data, rate 1.0, 0, 0x7fffffff, flags 0x16)` - kTuneLoopUntil
  | kTuneExcludeEdgeNotes | kTuneDontClipNotes: it loops for ever. Time
  scale QuickTime's default, 600 units a second. `SetMusicVolume(v)`:
  `TuneSetVolume(v − 3000)` (0 below); the volume setting (prefs +0xe, 0–9)
  × 0x1c71. `FadeMusic(out/in, ticks)`: 24 steps of `ticks`. `MusicIdle`
  calls `TuneTask` every 40th time. `SetMusic(-1)`: volume 0. Without
  QuickTime (or music off, prefs +0xb) no music.
- **Header**: a `musi` sample description (size, 'musi', reserved, data
  ref index 1, 1), then one general event per part (head `0xF` + part
  (12 bits) + length in words; tail `0b11` + type (14) + length), ended by
  the marker `0x60000000`. Type 1, note request: flags, polyphony,
  typical polyphony (Fixed), then a ToneDescription - synthesizer type,
  synthesizer name, instrument name (Str31 each), instrument number, GM
  number. Type 6, atomic instrument: 12 bytes, then a QuickTime atom
  container `sean` with a `tone` atom (the same ToneDescription).
- Tunes 600, 601, 603, 604, 606, 607, 615, 616, 620 ask "Best Synthesizer" for
  General MIDI instruments (Flute #74, Standard Kit #0x4001, ...): the
  instruments of QuickTime Musical Instruments (Apple's extension, Roland
  GS sounds; not part of Harry). Tunes 605, 608–614 ask "QuickTime Music
  Synthesizer" for instruments of `Harry Instruments` by name ("Tech Drum
  Kit 1", "Bassics", ...; their numbers 0x808a… are the component's).
- **Events** (32 bit, big endian; 64 bit where noted): `000` rest (24-bit
  duration, the only thing that advances time); `001` note: part (5),
  pitch (6, + 32), velocity (7), duration (11); `010` controller: part
  (5), controller (8), value (16, signed 8.8); `011` marker: `0x60000000`
  ends the tune, `0x60000001` (234 of them) comes before a general event
  type 5 (tune difference, an empty state snapshot); `1001` extended
  note (64): part (12), pitch (16: a note number, or 8.8 when above 127) /
  velocity (7), duration (22); `1010` extended controller, `1011` knob
  (both 64, unused here); `1111` general event. Used: 23 996 notes, 1257
  extended notes, controllers 7 volume (0–127 in 8.8), 10 pan (256 left …
  384 middle … 512 right; 322–450 used), 32 pitch bend (±2 semitones in
  8.8), 33 (0–127 as an integer; aftertouch?), parts 0–8.
- **Instruments** (`ssai`, in `Harry Instruments` 512–546 and in
  QuickTime Musical Instruments, 235 by instrument number: GM 1–128,
  variations from 1025, drum kits from 16385 = 0x4001): a classic atom
  (size, 'ssai', 0), then an atom container (atoms: size, type, id,
  reserved, child count, reserved = 20 bytes): `sean` with `tone`, `knbl`
  (knob list: flags, count, (number, value)…), one `sinf` per key range
  (`sdsc`: format 'raw ' / 'twos', channels, bits, rate (16.16), sample
  data id, offset, count, loop type, loop start, loop end, normal pitch,
  lowest, highest key; optional `knbl`), and the samples - `sdat` by id in
  Harry's, `smin` (id) holding `sdat`, `snam`, `quid` in QuickTime's, where
  an instrument also uses samples kept in another instrument's `smin`.
  All samples are 8-bit mono.
- The port reads both (`harry_original::qtma`) into the data pack
  (`harry_data::music`): `music/tunes/ID.txt` (parts and events as text),
  `music/instruments/KEY.txt` (`harry-<ssai id>`, `qt-<number>`: name,
  knobs, zones), `music/samples/KEY.wav` (8-bit mono); a recording
  `music/recordings/ID.wav` / `.flac` replaces the synthesized tune.
  QuickTime Musical Instruments is read from `re/work/qtmi` (its
  resource fork, copied from the Mac OS 9 disk; another path as the
  extract tool's third argument) and is required: the pack always holds
  the instruments the tunes ask for (24) - decided 2026-11-04, Apple has
  no objection to shipping them. Tunes run 32 s (614) to 91 s (601) once
  through.
- **The port's synthesizer** (`crates/harry/src/music.rs`, in the audio
  callback at 22 254 Hz): the tune's events at 600 units a second,
  looping at the end with the notes still sounding (as kTuneLoopUntil +
  kTuneDontClipNotes); per note the zone of its key, the pitch from the
  zone's normal key, pitch bend (controller 32, ±2 semitones), the
  sample's loop while it sounds, velocity, part volume (7) and pan (10),
  linear interpolation, a release of 80 ms; at most 48 notes. Not yet:
  the knobs (envelopes, velocity curves, transposition), controller 33,
  polyphony limits, QuickTime's own mixing level - to compare with
  recordings of the original (SheepShaver has QuickTime). A recording of
  a tune in the pack is played instead. Volume: `TuneSetVolume(v ·
  0x1c71 − 3000)` with the setting v (0–9, default 7, in the options).
- Where the port plays music: the main menu 600; `GameLoop`'s start
  (fade out over 4, the story with the scroll music 620, `IntroLevel` -
  which fades the music out for any level but the world's first (Hwld
  +0x1c4) - then the level's music (Hlvl +0x120fa) fading in over 2);
  `UpLevel` (the stories with the scroll music, then the next level's);
  `Win` (the win music 616 over 6, out over 6 at the end); Harry's and
  the boss's death fade it out over 1. In the sequences the port waits
  for a fade as the original does.

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

The emulators, ROMs and system disks are shared by the Ouro ports in
`ouro-tools/` next to the repositories (see its README); the game's
disks and the emulators' settings stay in `re/work/emu` and
`re/work/emu-ppc`. Start from the repository root:

    # 68k build, System 7.5 (Basilisk II)
    ../ouro-tools/basilisk/BasiliskII/src/Unix/BasiliskII --config re/work/emu/basilisk.prefs
    # PowerPC build, Mac OS 9.0.4 (SheepShaver)
    ../ouro-tools/sheepshaver/SheepShaver/src/Unix/SheepShaver --config "$PWD/re/work/emu-ppc/sheepshaver.prefs"

### SheepShaver (PPC)

`ouro-tools/sheepshaver`: SheepShaver built from kanjitalk755/macemu
(`--disable-jit`, SDL2), ROM `ouro-tools/roms/newworld86.rom` (user), Mac
OS 9.0.4 CD (user, `691-2684-A`, linked as `ouro-tools/systems/
macos904.iso`) installed onto `ouro-tools/systems/macos9-harry.dsk` (2 GB,
Mac OS Extended; "Update Apple Hard Disk Drivers" switched off; clean
copy `macos9-clean.dsk`), Harry installed by running the installer again
under OS 9 (puts Harry Instruments into its System Folder),
`re/work/emu-ppc/harry.dsk` = copy of the Basilisk disk.
`re/work/emu-ppc/sheepshaver.prefs`: 256 MB, 640 × 480, `bootdrive 0`
(−62 for the CD). It runs at full speed for the user; the slow input
during my installation was the installer / first boot (or my
automation), not the emulator in general. Once (2026-10-23) it quit
silently during play (no crash report, no log message) - right after
test key events (Shift + = + Page Up via Quartz); cause unknown.

### Basilisk II (68k)

`ouro-tools/basilisk` (built by S.C.Out's `re/tools/emulator/build.sh`),
Performa ROM `ouro-tools/roms/PERFORMA.ROM`, the System 7.5 + Sound
Manager 3.1 boot disk with `Harry Instruments` in Extensions
(`ouro-tools/systems/system75-sound-harry.dsk`); in `re/work/emu`:
`harry.dsk` (100 MB HFS) with the installer, `harry-installed.dsk` =
after installation, `basilisk.prefs`. 64 MB RAM, 640 × 480 × 8. Clicks
must be press - pause - release (cliclick `dd`/`du` with 100–150 ms),
instant clicks are ignored. The boot disk has no QuickTime yet: music
needs QuickTime 2.5+ with QuickTime Musical Instruments.

### systemless (no ROM, no System)

systemless 0.109 (`brew install benletchford/tap/systemless`; a Toolbox
of its own in Rust, see the skill's `references/related-projects.md`)
runs the PowerPC build from the folder of MacBinary files, headless, with
a JSON play script: 28 s of game in about 4 s. `re/tools/sl_repro.py
SAVE [STEPS]` builds `re/work/systemless/harry` (the installed files, one
`ouro-harry-repro` save - the Continue dialog takes the first file), goes
through the menu's Continue Game and Open, and takes screenshots
(`frames:N` one a tick; the screen is 800 × 600 with the game at (80,
60)). The application is unchanged; a licence file in systemless's
Preferences folder (`<game folder>/System Folder/Preferences`, also in
the zip of `sl_peek.py`) registers it - unregistered, Continue refuses
levels after 5. The menu then reads "This copy of Harry is licensed to:
Ouro Test". (Before: the check `FUN_10057cb0`, file offset 0x59330 in
the MacBinary, patched to `li r0,1; stb r0,0(r3); li r3,0; blr`.)

### Registration (`FUN_10057cb0`)

The check reads "Harry License" from the Preferences folder (FindFolder
'pref'; the name is STR# 900 #1 "Harry" + " License"; created empty
with type 'Areg', creator 'Alic' if missing - the `Register Harry`
utility writes it): 0x202 bytes, the licensee as a Pascal string at 0,
the code at 0x100, the number of copies (word) at 0x200. An empty file
gets the default from `FUN_10058a24` (STR# 900: "NOT REGISTERED",
"N/A"). `FUN_10059074` computes the code: the name upper-cased (ASCII
a-z, `FUN_10058f9c`), a 32-bit sum from 0 over the name's bytes, then
"Harry" (`FUN_10058fd8`: low byte += byte, rotate left 5, low word +=
copies), then eight letters 'A' + low nibble, rotating right 4 after
each. `EqualString` compares it with the stored code, case-sensitive.
After reading, the check writes the handle back to the file
(`FUN_10058c94`). With the flag `*_DAT_10066238` clear it returns
−1001 and "unregistered" without looking. It is the old-style Ambrosia code (variant 1) that
siddhartha77's BSNG plugin "Ambrosia Software" also computes.
`re/tools/harry_license.py NAME [COPIES] [--write DIR]` gives the code
and writes the file; `re/tools/emu_license.py` checks it against
`FUN_10059074` in Unicorn (3663 names and copy counts, all equal; a
planted bug in the upper-casing shows as 666 differences). Checked in
systemless: the unchanged application with the file is registered and
continues in level 6; one wrong letter, and it is unregistered again.
For SheepShaver the same file goes into its System Folder's
Preferences (type and creator through the shared folder's Finder info).
What systemless gets wrong (its Toolbox, not Harry's): the game area is
striped while a balloon waits, and the panel's level name is drawn with
its letters spread out ("T  e  m"). Otherwise the screens match the
port's (level 1, level 11, the meters). For looking, never as the
reference: the differential tests stay in Unicorn.
The Open dialog takes the first name and sorts without case, so the save
goes in as `Repro <name>` (a name after "System Folder" opened that
folder instead). The fixed clock gives the same random numbers every
run: the first use of a mystery vial is always the same outcome.
**Reading globals** (`re/tools/sl_peek.py SAVE [STEPS] [--read
NAME=ADDR]`): a systemless built with its debugger socket
(`ouro-tools/systemless`, `cargo install systemless --version 0.109.0
--locked --features debug-server`; Homebrew's has none) runs the steps
as a `--tick-input-script` (the socket mode cannot take a folder: the
game goes in as a zip, the save as MacBinary), pauses, and reads memory.
The data section sits at 0x2000000 there (found by the B-E-N text at
+0x3718); a global at Ghidra address A is at 0x2000000 + A − 0x10065390,
and a `@` global in `re/ghidra/globals.txt` is a TOC slot holding a
pointer to it (the ammunition counters). r2 when paused is not always
the game's TOC (it may stop in a library).

## Port and differential tests

- Rust workspace: `crates/harry-core` (game logic, no I/O; doc comments
  name the original routine), `crates/harry-data` (the data pack),
  `crates/harry-original` (the original's formats, `ouro-harry-extract`
  and the tests against the original), `crates/harry` (the game with
  SDL3, `ouro-harry`). `cargo test` runs the tests; those that need
  recordings or the original's files skip when `re/work/` lacks them.
- **Data pack** (`harry-data`; `data/ouro-harry-data.zip` in the
  repository, built into the game, made by `cargo run --release -p
  harry-original --bin ouro-harry-extract -- re/work/installed
  data/ouro-harry-data.zip [path of QuickTime Musical Instruments.rsrc,
  default re/work/qtmi/]`; the same files give the same bytes): a zip of
  PNGs, WAVs and text - the 929 faces with
  their header rects, the 9 tile sets of Episode 1 (foreground with its
  mask as transparency), the collision masks (`cicn` 290, 299, 571–574,
  598, 599, 1299; the wall sheet PICT 160), `Harry Titles` PICT 129 (the
  game screen) and 150 (the meters), the meters' rounded ends (cicn
  10135–10137, `icons/`), the world's pictures and end fields (`world/`,
  `world.txt`; pack format 3), and the 15 levels as text (all
  decoded fields; the undecoded bytes, empty in Episode 1, are left out).
  Colours are `clut` 8 (`harry_data::clut8`); the tile set PICT 145's own
  table maps exactly. `tests/pack.rs` checks the pack against the
  original files (levels, faces, face masks, masks).
  The pack's levels keep no custom sprite tables (empty in Episode 1):
  `Level::custom` gives zeros for them, so copier saves work on pack
  levels (they crashed the port before).
- **Saves for the original's bugs** (`ouro-harry-repro <pack> <folder>`):
  copier saves that put Harry where a bug shows, with the meters,
  ammunition and items it needs - as the original's file (resource
  `HrSG` 0 in the resource fork, type `HrSG` / creator `Hary` in the
  Finder info, as the emulators' shared folders keep them on macOS) and
  as `.hrsg` for the port (`HARRY_SAVE=<file>`, then Continue or
  `--screenshot`). BUGS.md says per bug which save and what to do.
- **Frontend** (`ouro-harry [pack] [level]`; `--screenshot out.bmp
  [level] [frames]` renders without a window): `harry_core::game` at 30
  frames per second with the default key bindings (SDL keys mapped to Mac
  key codes), drawn into `clut` 8 indices as the original draws: screen
  picture, background tiles, background props, sprites with +0x8b, the
  foreground props, foreground tiles, the other sprites; the meters from
  PICT 150 (`UpdateMeters`: the empty strip, then the value's strip up to
  the shown value, 3 px shorter above 8 where a round end is drawn - cicn
  10135–10137; see "Meters"). A new game starts as `InitGameGlobals` (meters
  72). Level 1 looks as in the emulator.
- **Texts in the frontend**: the system's Geneva and Helvetica (or
  substitutes) at 9 pt = an em of 9 screen pixels, drawn at the window's
  resolution over the picture. The panel (`UpdateText`): the level name
  at (483, 118), Helvetica 9, grey 32000/65535; the score bold at (483,
  141), counting up 50 a frame toward the real score (a lower one at
  once), its grey 32000 + 40 × the distance (at most 800).
- **Balloons** (`ShowBalloon`): TextEdit in Geneva 9 within 195 px; the
  balloon 210 px wide and (lines + 1) × line height high (12 px: measured
  on the emulator), at screen (sprite at.h − view.h − 57, sprite at.v −
  view.v + 17 − height) - over Harry (top-left − 49, + 13) in mode 2;
  shown only if left and top ≥ 32, right ≤ 448 and bottom + 8 ≤ 448.
  Drawn: the game area dimmed with a gray pattern, a white round rect
  (16 × 16 corners) with a tail (l + 96 … l + 104 at the bottom down to
  (l + 88, b + 8)), the text black at (l + 5, t + 5). The game waits:
  Return / Enter close it at once, any key after 60 ticks. The port
  measures with the system's Geneva, so the line count of a text can
  differ from the bitmap font's (the level 1 memo breaks the same).
  The world's 87 texts come from `Hwld` 0 (`texts.txt` in the pack).
- **Sound**: `STPlayRegSound(snd, priority, volume)` - volume 0x100 =
  full (scooting: wetness / 2, floor sounds 0x80 − wetness / 2);
  `STPlay3DSound(snd, priority, volume, at)` and `…Pitched` (a Fixed rate,
  0x10000 = as recorded: water 55000, the balloon 56000 + `SATRand(16000)`,
  the gulp 50000). `CalcStereoVolume`: an ear at each side of the view (h
  and h + 448, at the middle's height), per ear (1024 − `PEDistance`) / 8
  of 128 within 1024 px, times the volume. All 103 `snd`s (100 in `Harry
  Sounds`, 200 / 201 / 203 in the episode for levels with their own
  sounds: `LoadLevelSounds` loads them with the world file first) are
  8-bit mono, 11025 / 22050 / 22254 / 5563 Hz, format 1 (one format 2).
  Sounds the routines name by event: chatter 150–152 (`SATRand(3)` 0 →
  150, 1 → 151, 2 → 152), `Ouch` 220 / 225 / 226 (`SATRand(100)` < 43,
  < 86, else), scoot 200 / 202 / 201 (floor kind 1 / 2 / 3), wet scoot
  215, kick hit 203, half pipe 209, landing 207, bounce 208, falling into
  a hole 4901, back from it 224, water 215, balloon 217; plants use 1101
  for all three of their jump sounds. `harry_core::game` collects a
  frame's sounds (`sound::SoundEvent`: volumes per ear, rate, the
  original's voice / chatter / remark / charge channels); the frontend
  mixes up to 8 at 22254 Hz stereo. Falling into a hole also moves the
  view onto Harry (`JumpScale` frame 1030) - now applied by the game.
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

## Port improvements

Where the port deliberately differs from the original (the game logic in
`harry-core` stays faithful; these are around it):

- **Keyboard layout.** The original reads key positions (`GetKeys`, US
  layout): on a German keyboard the cheat W-A-V-Y-G-R-A-V-Y is typed
  W-A-V-Z-G-R-A-V-Z. The port's frontend gives the letters by the current
  layout (`Keycode::from_scancode`, `LETTERS` in `ouro-harry`), so the
  letters printed on the keys work. The other keys (arrows, space,
  modifiers, digits, F keys) stay by position.
- **Skipping a scrolled picture.** The original ends `ScrollPlot` on any
  key or mouse button that is *down* once 8 rows are shown - a key still
  held from playing (an arrow at the level's end) skips the story by
  accident. The port needs a press made during the picture (a new key
  or mouse button event, key repeat ignored); a press in the first 8
  rows ends it with the 8th.
- **Menu.** Letters by the layout as in the game; the credits close
  with a key too (the original: only a click in them); a button taken by
  key is drawn released again; Escape in the menu quits; no registration
  line under the menu. Level jumps for testing are F7 / F8 (N and P are
  needed by the key sequences).
- **Options.** A screen of the port's own over the menu instead of the
  preferences dialog (`crates/harry/src/options.rs`): camera mode (the
  dialog's four), save between levels, sound volume (off, 1–9), scaling
  (whole multiples or filling the window), full screen and its key (F11;
  any key the game does not use), the pause key (caps lock), the seven
  keys
  (Return, then the key; a key already used swaps), default settings.
  Left out: reduce frame rate, line skipping, interlace, more compatible
  drawing, allow system tasks (speed options of 1997); the music volume
  comes with the music. Saved in `settings.txt` in SDL's preference
  folder. The defaults are the original's (`InitPrefs`: camera 1 =
  Constant, Catch-up; sound 9, music 7; keys ← → ↓ ↑, space, control,
  option, Tab for the map).
- **Music fades in the game.** `FadeMusic` waits for its 24 steps: at
  Harry's death (frame 0) and the boss's the original stops for 0.4 s.
  The port fades while the game goes on (in the sequences between the
  frames it waits as the original).
- **Recordings.** A pack may bring a finished recording of a tune
  (`music/recordings/ID.wav` / `.flac`, e.g. a fan pack); it is played
  instead of the synthesized tune.
- **Pause.** The pause key is a setting (caps lock by default, paused
  while it is locked as in the original; another key pauses and goes on
  by turns), and the game also pauses when its window loses focus (any
  key or click goes on; the pause picture's "Press CAPS LOCK" line then
  says what to press). No menu bar or cursor is shown.
- **Registration.** The port plays as registered; the shareware limit
  after level 5 (`LevelLimit`) is left out.
