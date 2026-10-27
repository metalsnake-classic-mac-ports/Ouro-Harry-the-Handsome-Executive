# TODO

## Phase 1: data
- [x] Install 1.0.1 from the VISE installer in Basilisk II, take the files out
- [x] Unpack the application (`re/work/app`), list all resources (`re/work/res`)
- [x] MacsBug symbols of the 68k code (`re/tools/macsbug_syms.py`)
- [x] Decode the rest of `Hlvl` (custom sprite tables stay raw)
- [ ] Decode `Hwld`
- [x] Render all levels to PNG (`re/tools/render_level.py`), matches the emulator
- [x] 1-bit PICT masks (in render_level.py)
- [x] `HrSp` encoded faces → PNG (`re/tools/hrsp.py`)
- [x] Screen palette: clut 8
- [ ] Map object ids → sprite faces (which HrSp per kind) for a full level picture
- [ ] `Tune` format (QTMA) and `ssai` instruments; plan for the music
- [ ] Compare with 1.0.0 (older installer format) - worth it?

## Phase 2: code (PPC is the reference)
- [x] Ghidra: PEF import (PowerPC:BE:32), names from traceback tables, decompile all (`re/work/ppc/harry.c`)
- [x] Python venv with capstone + unicorn (`re/work/venv`, PPC32 works)
- [ ] Name the SAT routines in the PPC build (SAT source, 68k CODE 7 names)
- [ ] Name the tile scroll routines (68k CODE 6: WRAPCOPY, SCROLLTO, ...)
- [x] Ghidra pipeline `re/ghidra/run.sh`: TOC fix, TOC/TVector names, globals list, Sprite struct
- [ ] Keep naming globals (`re/ghidra/globals.txt`) and sprite fields (`re/ghidra/sprite.txt`)
- [x] Unicorn harness for PPC routines (`re/tools/ppcmachine.py`: imports → Python stubs)
- [x] Tile shape tables captured from `InitTileHotRects` (`re/tools/emu_tables.py`)
- [ ] Later: CodeWarrior 68k far-model loader for differential tests against Basilisk II
- [x] Main loop (`GameLoop`), timing: 30 Hz logic
- [x] Player movement: keys, scoot/inch/kick tables, impulse by direction, friction, max speed
- [ ] Player collisions: `HitPlayerTileSprite` (walls, tile kinds), `HitPlayerSprite`, `CheckTileCollisions`
- [x] Tile kinds and `CheckTileCollisions` order; player's reaction per background kind
- [ ] `WallBounce`, `BGTileBounce`, `RectBounceFake` (the bounce maths), mask test `FUN_10054124`
- [ ] Ramps, stairs, half pipes (`HandleRampCollision`, `HandleStairsCollision`, `HandleHalfPipe`, `JumpScale`)
- [ ] Meters: what changes comfort / favor; death and respawn (`HandlePlayerSpriteDeath`, copier saves)
- [ ] SAT run order (`HandleSprites`, idle sprites) - which sprite moves first
- [x] Random generator: `SATRand(n)` = |Toolbox Random| % n, seeded from the clock
- [ ] Sprite kinds one by one

## Phase 5: port (Rust)
- [x] Workspace: `harry-core` (no I/O), `harry-original` (tests against the original)
- [x] Physics: impulse by direction, max speed, floor friction; moves (scoot, kick, inch) -
      differential tests against the PPC routines (`re/tools/emu_physics.py`, 16 000 cases, planted bugs caught)
- [x] `WallBounce` (mask test as a parameter), differential test 12 000 cases
- [x] `RectBounceFake`, differential test 12 000 cases (incl. exact diagonal ties)
- [x] `BGTileBounce` with real levels in Unicorn (`Machine.load_level`), 12 000 cases
- [x] Region mask test + `REGIONSEPARATE` with the real masks (12 000 cases)
- [x] Full chain: `WallBounce` + real mask test (`emu_mask.py re/work chain`), 12 000 cases
- [x] Same for `BGTileBounce` (obstacle mask cicn 290), 12 000 cases
- [x] Mirrored face masks confirmed with the original's BuildMaskRegion in Unicorn (all 929 faces)
- [x] Tile collision pass (`CheckTileCollisions`) and the player's tile reactions (19 500 cases)
- [x] Half pipes, jumps, holes (`HandleHalfPipe`, `JumpScale`), 46 000 cases
- [x] Whole player frame (`HandlePlayerSprite`) without keys, 20 000 cases
- [ ] `HandleKeys`: turning, moves, items, weapons (player shots), cheats and key sequences
- [ ] See the garbage-scale frame (BUGS.md) in the emulator: drop Harry into a hole in level 11
- [x] Level model (`harry_core::Level`), MacBinary / resource fork / `Hlvl` reader + writer (exact round trip)
- [x] Graphics readers in Rust: PICT 1/8-bit/direct, cicn, HrSp; masks from the original files
- [ ] `harry-data`: data pack; extract tool in `harry-original`
- [ ] Frontend (SDL3)

## Phase 3: emulator
- [x] SheepShaver built (`re/work/emu-ppc/src`), Mac OS ROM (newworld86.rom) from the user
- [x] Mac OS 9.0.4 installed in SheepShaver, Harry installed there
- [x] SheepShaver: "Couldn't open Sound Tool!" - fixed by selecting the output device in the Sound control panel; PPC build runs
- [x] 68k original runs in Basilisk II: menu, prefs, level 1 (screenshots in re/work/screens)
- [ ] Music in Basilisk: QuickTime 2.5 + Musical Instruments on the System 7.5 disk
- [x] Screen palette: clut 8 (from the code; 66 of 68 PICTs already in clut 8)
- [ ] Harry-big Easter egg key (0x74?): short press of Shift + = + Page Up
