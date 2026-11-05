# TODO

## Phase 1: data
- [x] Install 1.0.1 from the VISE installer in Basilisk II, take the files out
- [x] Unpack the application (`re/work/app`), list all resources (`re/work/res`)
- [x] MacsBug symbols of the 68k code (`re/tools/macsbug_syms.py`)
- [x] Decode the rest of `Hlvl` (custom sprite tables stay raw)
- [ ] Decode `Hwld` (the texts at 0x3c9 + n · 256 are done)
- [x] Render all levels to PNG (`re/tools/render_level.py`), matches the emulator
- [x] 1-bit PICT masks (in render_level.py)
- [x] `HrSp` encoded faces → PNG (`re/tools/hrsp.py`)
- [x] Screen palette: clut 8
- [ ] Map object ids → sprite faces (which HrSp per kind) for a full level picture
- [x] `Tune` format (QTMA) and `ssai` instruments: read into the data pack (tunes as text, instruments, 8-bit samples; recordings optional)
- [ ] Compare with 1.0.0 (older installer format) - worth it?

## Phase 2: code (PPC is the reference)
- [x] Ghidra: PEF import (PowerPC:BE:32), names from traceback tables, decompile all (`re/work/ppc/game.c`)
- [x] Python venv with capstone + unicorn (`re/work/venv`, PPC32 works)
- [ ] Name the SAT routines in the PPC build (SAT source, 68k CODE 7 names)
- [ ] Name the tile scroll routines (68k CODE 6: WRAPCOPY, SCROLLTO, ...)
- [x] Ghidra pipeline `re/ghidra/run.sh` (the skill's `run_ghidra.sh`): TOC fix, TOC/TVector names, globals list, Sprite struct
- [ ] Keep naming globals (`re/ghidra/globals.txt`) and sprite fields (`re/ghidra/sprite.txt`)
- [x] Unicorn harness for PPC routines (`re/tools/ppcmachine.py`: imports → Python stubs)
- [x] Tile shape tables captured from `InitTileHotRects` (`re/tools/emu_tables.py`)
- [ ] Later: CodeWarrior 68k far-model loader for differential tests against Basilisk II
- [x] Main loop (`GameLoop`), timing: 30 Hz logic
- [x] Player movement: keys, scoot/inch/kick tables, impulse by direction, friction, max speed
- [x] Player collisions: `HitPlayerTileSprite` (walls, tile kinds), `HitPlayerSprite`, `CheckTileCollisions`
- [x] Tile kinds and `CheckTileCollisions` order; player's reaction per background kind
- [x] `WallBounce`, `BGTileBounce`, `RectBounceFake` (the bounce maths), mask test `FUN_10054124`
- [x] Ramps, stairs, half pipes (`HandleRampCollision`, `HandleStairsCollision`, `HandleHalfPipe`, `JumpScale`)
- [x] Meters: what changes comfort / favor; death and respawn (`HandlePlayerSpriteDeath`, copier saves)
- [x] SAT run order (`HandleSprites`): tasks, sort pass, collision pass, player collisions - 20 000 cases
- [x] Idle sprites (`HandleIdleSprites`) and level start (`SetupLevelSprites`, `GenerateSprite`) in the port; SAT records with the free list (3000 cases)
- [x] Sprite record in the port (`sprite::Sprite`), faces by HrSp id; bonus, item, exit (9 000 cases)
- [x] `HitPlayerSprite` for bonus and item (12 000 cases)
- [x] Door (kinds + `HitPlayerSprite`, 3 000 + 30 000 cases)
- [x] Water cooler (12 000 cases)
- [x] Plant movement and tiles (`HandleGlide`, 12 000 cases)
- [x] Plant hit task with Harry, plant, cooler, doors, shots; `RectBounce` (20 000 cases)
- [x] Desks (301–319; none in Episode 1): setup, Harry's mask test and bounce, the plant's, barrel's, chair's, robot's and reversor's branches (recorders extended)
- [x] Barrel and chair: setup, task, tiles, hit tasks; in the plant's, enemy shots' and fans' tests (52 000 cases)
- [x] Stairs: setup, `HandleStairsCollision` for Harry, plants, barrels, chairs (30 000 cases)
- [x] Buttons and sliding doors (30 000 runs), sliding doors in the Harry, object and shot recorders
- [x] Co-workers (15 000 runs), as partners of Harry and the objects
- [x] Mail guy (6000 runs), its crash with objects, Harry run over; levels 2 and 3 have all their kinds
- [x] `STPlayDynamicSound` / `UpdateDynamicSounds`: the sound's stereo position moves with the mail guy
- [x] Robots, small and big (8000 runs, 30 000 hits), in the partner recorders; level 4 has all its kinds
- [x] Fire pipes and flames (30 000 runs); level 5 has all its kinds; the world recorder covers every ported kind
- [x] Grates, fences and ghosts (75 000 cases); level 6 has all its kinds
- [x] Turrets, reversors, boulders (18 000 runs); level 7 has all its kinds
- [x] Scientists and blobs (16 000 runs), player shot hits on every kind (30 000); levels 8–14 have all their kinds
- [x] The boss (4000 runs); every level of Episode 1 has all its kinds (hazards 3001 and desks 301–319 are not used)
- [x] Music (`Tune`), the boss's and Harry's `FadeMusic`
- [x] Whole-frame recorder (`emu_game.py`) for all 15 levels with the draw pass and every draw proc's effect (1500 runs)
- [x] `VendingDrawProc`: drawing the vending machine sets its +0x78 (and +0x8a)
- [x] Message sprite (`ShowBalloon` as a callback; 12 000 cases)
- [x] `ShowBalloon` / `GenerateBuzzwords` (frontend: text layout, modal wait); world texts from `Hwld`
- [x] Colliders: fans (`HandleRepulsions`), phone, copier, magnet, wall guns, vending machine, gate (20 000 cases)
- [x] `HitPlayerSprite` for colliders (`RectBounce` once a frame, gate key, vending kick; 30 000 cases)
- [x] Dart enemy (+ `HitPlayerSprite`), props (15 000 cases)
- [x] Ramp: setup, task, `HandleRampCollision`, `HitPlayerSprite` (50 000 + 30 000 cases)
- [x] Pushable ramps: `HitRampSprite` (`RectBounceFake2`), `HitRampTileSprite` (`WallBounce2`, `BGTileBounce2`), 24 000 cases
- [x] `EraseSprites`: +0x94 / +0x96 countdowns, frozen sprites leave, level entries updated
- [x] Whole frame of level 1: player and held weapon creation, `FindUpperLeftCorner`, task dispatch, `CheckPlayerCollision`, `CheckTileCollisions` for all sprites (600 runs of up to 300 frames)
- [x] Whole frames with weapons and items: player shots as records, `HitPlayerShotSprite`, effects (explosions), 900 runs
- [x] Harry's death (`HandlePlayerSpriteDeath`) to the end of the game loop (3000 runs + whole frames)
- [x] Saved games: the record, `DoSaveGame` (autosave), `DoCopierSaveGame`, `ContinueGame` (whole frames)
- [ ] Saving and loading as a list of saves (later, decided 2026-11-03): no file dialogs; a save at a copier or between levels makes a new entry with level, date and time, a screenshot; continuing picks from the list. Replaces `CopierSave` / `AskToContinue` and the original's save files
- [x] `UpLevel`: next level in the same loop, score, level-start state (whole frames, 37 level changes)
- [x] `Win`, `EndLevelEffect`, `ScrollPlot` pictures, `IntroLevel`, gamma fades (frontend sequences, timing tested)
- [ ] `IntroLevel`'s text card for levels without a title picture (fan worlds)
- [x] Main menu: buttons by mouse and key, instructions, credits, the X message; back to it after `Win`, a death and Escape (with a question)
- [x] The inventory on the panel: slots, selection, item faces, counts, ammunition bars; `UpdateAmmo`'s clamp in the core
- [x] Options as a screen of the port's own: camera, saving between levels, sound volume, scaling (integer or free), full screen, keys
- [x] The map (`ShowMap`) with its key in the options; the full screen key in the options
- [x] Pause (caps lock or a key of the options, and on focus loss), the options during a game (Command-I)
- [ ] The warp dialog
- [x] The ripple: phase, cheat, `BuildSineTable` (7865 cases), row shifts in the renderer
- [x] Enemy shots: setup, flight, tiles, hits (colliders, coolers, plants, shots), `FindDesiredDirectionGeneric` (57 000 cases)
- [x] Random generator: `SATRand(n)` = |Toolbox Random| % n, seeded from the clock
- [x] Sprite kinds one by one

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
- [x] `HandleKeys` with the inventory routines: turning, moves, items, weapons, cheats and key sequences, 30 000 cases
- [x] Player shots and the held weapon (`MakeNewShot`, `HandleHeldWeaponSprite`, shot flight, shots vs. walls), 47 500 cases
- [x] Shots hitting sprites (`HitPlayerShotSprite`): `re/tools/emu_playershot.py`
- [ ] Watch the original's bugs with the `ouro-harry-repro` saves (systemless: `re/tools/sl_repro.py`, values with `re/tools/sl_peek.py`; seen: meter-end, backfire, vial; then SheepShaver for what systemless may draw wrong) (BUGS.md, "See it"; next the shots passing walls and the garbage-scale frame, which needs a jump over a hole) and note what is seen
- [ ] Saves for the bugs still without one: two pushed ramps meeting (level 14), two robots at a wall
- [x] Level model (`harry_core::Level`), MacBinary / resource fork / `Hlvl` reader + writer (exact round trip)
- [x] Graphics readers in Rust: PICT 1/8-bit/direct, cicn, HrSp; masks from the original files
- [x] `harry-data`: data pack; extract tool `ouro-harry-extract` in `harry-original`
- [x] Frontend (SDL3): level 1 drawn and playable at 30 fps (`ouro-harry`)
- [x] Frontend: panel texts (level name, score), balloons (world texts, buzzwords, phone)
- [x] Frontend: sound (all `snd`s, 3-D volumes, rates, channels)
- [x] Sprite tints: the draw procs' blits (whole frames), the colour tables (13 812 colours), drawn in the frontend
- [x] Meters as `UpdateMeters` draws them, with the rounded ends (cicn 10135–10137 in the pack); the fans' grilles and the dart enemies' darts
- [x] Music: the synthesizer (tunes from the pack, a recording instead when there is one), `SetMusic` / `FadeMusic` where the game calls them, the volume in the options
- [x] Sound feedback into the game: the frontend reports what plays, the game answers the original's "is it playing" questions from it (voices, chatter, gulp, fan, bumps, water, machine gun, boss)
- [x] Frontend: letters by the keyboard layout (an improvement, NOTES "Port improvements"); F1–F4, F12, F13, N and P mapped
- [x] `KeyEvent::Balloon`: the P-A-R-G-H-Z and B-E-N balloons (texts from the PowerPC data section)
- [ ] `Hwld` 0: decode the fields before the texts (0–0x3c8)

## Phase 3: emulator
- [x] SheepShaver built (`re/work/emu-ppc/src`), Mac OS ROM (newworld86.rom) from the user
- [x] Mac OS 9.0.4 installed in SheepShaver, Harry installed there
- [x] SheepShaver: "Couldn't open Sound Tool!" - fixed by selecting the output device in the Sound control panel; PPC build runs
- [x] 68k original runs in Basilisk II: menu, prefs, level 1 (screenshots in re/work/screens)
- [ ] Music in Basilisk: QuickTime 2.5 + Musical Instruments on the System 7.5 disk
- [x] Screen palette: clut 8 (from the code; 66 of 68 PICTs already in clut 8)
- [ ] Harry-big Easter egg key (0x74?): short press of Shift + = + Page Up

## Release
- [ ] First release: the repository on GitHub, GitHub Pages from `docs/`,
      CHANGELOG cut to 0.1.0, tag `v0.1.0`
- [ ] Homebrew: formula `ouro-harry` in the tap `homebrew-games` (url of
      the tag's tarball, its sha256, `license "GPL-3.0-or-later"`, `head`
      on `main`; tested with a local tarball on 2026-11-05: builds,
      `brew test` and `brew audit --strict` pass), pull request, bottles
      with the "brew pr-pull" workflow, the tap's README table
- [ ] Build and test on Windows and Linux (so far built only on macOS)

## Lowest priority (decided 2026-11-04)
- [ ] `MakeITable`'s inverse table exactly (the port takes the nearest colour); second faces clipped like the original at the ring buffer's wrap
- [ ] Music closer to QuickTime: the instruments' knobs (envelopes, velocity curves, transposition), controller 33, the mixing level - against recordings from SheepShaver
