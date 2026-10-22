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
- Reproduce: `re/ghidra/` scripts, headless:
  `analyzeHeadless re/work/ghidra-proj harry-ppc -import Harry.pef
  -processor PowerPC:BE:32:default -postScript ApplyNames.java
  traceback.txt -postScript ExportFunctions.java functions.txt
  -postScript DecompileAll.java harry.c` → `re/work/ppc/` (1043
  functions, 49 500 lines of C).

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

- 0x0000–0x7fff: **background grid**, 128 × 128 big-endian words, row
  major. Values 0–14 are the random floor variants; higher values are
  special floor tiles.
- 0x8000–0xffff: **foreground grid**, 128 × 128 words. 60–63 fill the
  area outside the level, 0xffff = no tile, 0–59 walls etc.
- Tiles are 32 × 32 → the world is 4096 × 4096 pixels.
- 0x10000–0x10fff: **sprite list**, 256 records of 16 bytes
  (`SetupLevelSprites`): byte 0 = present, word +2 = **object id** (the
  editor's ids: 0x0385 = 901 door up, 0x0259 = 601 plant, 0x23f0 = 9200
  copier, 0x01f4 = 500 water cooler, 0x2707 = 9991 fade exit, 0x5208 =
  21000 ?), words +4…+a = info fields, +c and +e = position (pixels);
  the sprite is generated at (+e − 6, +c − 6) with its record index.
- 0x11000, 0x11340: tables (to be decoded).
- 0x114c0: **level name**, Pascal string ("Welcome to ScumCo"); also
  compared against fixed strings somewhere (cheat/special levels?).
- 0x12032/0x12034: ?
- 0x12074: **level info fields #0–#15**, words (see list below). Level 1:
  #6 = 200 (story picture scrolled before the level), #8 = 301 (picture
  shown before). Level 3: #5 = 201.
- 0x1209c–0x120b1: nibble arrays (`1111 1111 1111 1111 2222 2333`, `6666`).
- 0x120f4/0x120f6: two coordinates (level 1: 2082/2087) - start position?
- 0x120f8: ?; 0x120fa: **music id** (601, 603, ...); 0x120fc–0x120fe: ?
- 0x12100: physics override flag; 0x12102–0x1210a: five physics values
  (defaults when the flag is 0: 50, 70, 245, 252, 150; `LoadLevelPhysics`).
- Level −1: unknown purpose (test level? level select was disabled).

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

### Sprites `HrSp` (Harry Graphics, ids 1001–6054, 929 faces)

Named "Harry Sprite Face". Header: rect (0, 0, 44, 44) twice, then
encoded rows: opcode words like `0100 0030` (row start, length) and `0200
002c` (copy 44 pixels) followed by 8-bit pixels - an RLE "encoded face"
drawn by `BLITENCFACE…` (CODE 2). Ids in groups of ten/hundred per
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

`re/work/emu`: Basilisk II from the S.C.Out work folder (binary used in
place), Performa ROM, a copy of the System 7.5 + Sound Manager 3.1 boot
disk (now with `Harry Instruments` in Extensions), `harry.dsk` (100 MB
HFS) with the installer, `harry-installed.dsk` = after installation.
64 MB RAM, 640 × 480 × 8. Clicks must be press - pause - release
(cliclick `dd`/`du` with 100–150 ms), instant clicks are ignored.
The boot disk has no QuickTime yet: music needs QuickTime 2.5+ with
QuickTime Musical Instruments.
