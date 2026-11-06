# TODO

Open work only; what is done is in NOTES.md, BUGS.md, CHANGELOG.md and the
git history.

## Game
- [ ] The warp dialog
- [ ] Saving and loading as a list of saves (decided 2026-11-03): no file dialogs; a save at a
      copier or between levels makes a new entry with level, date and time, a screenshot;
      continuing picks from the list. Replaces `CopierSave` / `AskToContinue` and the original's
      save files
- [ ] `IntroLevel`'s text card for levels without a title picture (needed for fan worlds)

## The original's bugs
- [ ] Watch them with the `ouro-harry-repro` saves (systemless: `re/tools/sl_repro.py`, values
      with `re/tools/sl_peek.py`; then SheepShaver for what systemless may draw wrong) and note
      what is seen in BUGS.md ("See it"). Seen so far: meter-end, backfire, vial. Next: the shots
      passing walls and the garbage-scale frame (needs a jump over a hole)
- [ ] Saves for the bugs still without one: two pushed ramps meeting (level 14), two robots at a
      wall
- [ ] Harry-big Easter egg (Shift + `=` + key 0x74): try a short press in SheepShaver

## Reverse engineering
- [ ] `Hwld` 0: decode the fields before the texts (0–0x3c8)
- [ ] Name the SAT routines in the PPC build (SAT source, 68k CODE 7 names) and the tile scroll
      routines (68k CODE 6: WRAPCOPY, SCROLLTO, ...)
- [ ] Keep naming globals (`re/ghidra/globals.txt`) and sprite fields (`re/ghidra/sprite.txt`)

## Release
- [ ] Homebrew bottles for Intel Macs, as for S.C.Out
- [ ] Build and test on Windows and Linux (the Linux bottle builds; not played on Linux or
      Windows yet)

## Lowest priority (decided 2026-11-04)
- [ ] `MakeITable`'s inverse table exactly (the port takes the nearest colour); second faces
      clipped like the original at the ring buffer's wrap
- [ ] Music closer to QuickTime: the instruments' knobs (envelopes, velocity curves,
      transposition), controller 33, the mixing level - against recordings from SheepShaver
- [ ] Compare with 1.0.0 (the older installer in the fan archive) - worth it?
- [ ] CodeWarrior 68k far-model loader for differential tests against Basilisk II
