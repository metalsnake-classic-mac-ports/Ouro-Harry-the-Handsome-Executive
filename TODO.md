# TODO

Open work only; what is done is in NOTES.md, BUGS.md, CHANGELOG.md and the
git history.

## Game
- [ ] `IntroLevel`'s text card for levels without a title picture (needed for fan worlds)

## The rules for all ports (`../CLAUDE.md`, "How every port plays", 2026-11-12)
- [ ] What counts as a run's time: for now from the level's start to its exit, deaths
      included, the game's frames while playing (NOTES.md, "Port improvements"); confirm
      or change
- [ ] Game speed apart from the frame rate: the picture at the display's rate, the game at
      30 frames a second; sprites drawn between game frames
- [ ] Fan datasets: graphics in higher resolution and with more animation steps, sounds,
      levels and worlds with higher limits, a config file in the format shared by the
      ports; loading from the pack library (server side later)
- [ ] The original's own fan files as datasets (decided 2026-11-19): a custom world
      (`Hwld`, e.g. the ~27 in the fan archive) or a replaced data file (`Hrdt`:
      InvertedHARRY's `Harry Graphics`) added in the game library (ouro-framework) goes
      through the extract tool's conversion and is kept as an Ouro fan dataset (its config
      file filled from the world's info) in the settings folder; also by the extract tool
      on the command line. Harry is the only Ouro port so far whose original took such
      files (Quagmire, Sammy and S.C.Out had none)
- [ ] Gamepad (rule 10, decided 2026-11-12): stick / d-pad and buttons for Harry's
      actions, set on the keys page; the menus by gamepad too. Maybe later shared by all
      ports in a common Ouro framework

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
