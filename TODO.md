# TODO

## Phase 1: data
- [x] Install 1.0.1 from the VISE installer in Basilisk II, take the files out
- [x] Unpack the application (`re/work/app`), list all resources (`re/work/res`)
- [x] MacsBug symbols of the 68k code (`re/tools/macsbug_syms.py`)
- [ ] Decode the rest of `Hlvl` (after the sprite list) and `Hwld`
- [ ] Render all levels to PNG (tile sets + grids + sprites) as a format check
- [ ] 1-bit PICT masks (tile set mask PICTs)
- [ ] `HrSp` encoded faces → PNG (decoder from `BLITENCFACE…`)
- [ ] Screen palette: screenshot of the running game
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

## Phase 3: emulator
- [x] SheepShaver built (`re/work/emu-ppc/src`), Mac OS ROM (newworld86.rom) from the user
- [ ] Mac OS 8.6/9 boot disk for SheepShaver (from the user)
- [ ] QuickTime 2.5 + Musical Instruments on the boot disk, run the game
- [ ] Screenshots: menu, level 1, palette
