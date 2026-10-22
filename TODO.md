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

## Phase 2: code
- [ ] CodeWarrior far model: read CODE 1 startup, relocation + DATA decompression, build a loader (68k) - or decide for the PPC PEF
- [ ] Disassembler/decompiler setup for this layout (capstone/unicorn venv, Ghidra)
- [ ] Main loop (`GameLoop`), timing (frame rate, Reduce Frame Rate mode)
- [ ] Player physics (chair: scoot, kick, inch, glide)
- [ ] Sprite kinds one by one

## Phase 3: emulator
- [ ] QuickTime 2.5 + Musical Instruments on the boot disk, run the game
- [ ] Screenshots: menu, level 1, palette
