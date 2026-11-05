# <img src="docs/site/harry.png" alt="" width="32" height="32"> Harry the Handsome Executive
Mac game by Ben Spees and Ambrosia Software, 1997.
Website: https://metalsnake-classic-mac-ports.github.io/Ouro-Harry-the-Handsome-Executive/

![Level 1 of Harry the Handsome Executive in Ouro Harry](docs/site/screen-level1.png)

# About this project
Ouro Harry: a new implementation of Harry the Handsome Executive in Rust, for
macOS, Windows and Linux. The rules come from the original's PowerPC code
(version 1.0.1) and are checked against it: its routines run in Unicorn next
to the port, and both have to compute the same, case by case. The game data
comes from the original Mac version: `ouro-harry-extract` converts its files
into a data pack, `data/ouro-harry-data.zip` (PNG, WAV and text), which is
built into the game. The gameplay should stay as close to the original as
possible, with optional modern extensions.

Part of *MetalSnake's Classic Mac Ports*; "Ouro" (from Ouroboros) is the
name of these ports: the old game in a new skin.

Work in progress, see [TODO.md](TODO.md): all 15 levels of Episode 1 play.

## Building
You need Rust (via [rustup](https://rustup.rs)) and SDL3
(macOS: `brew install sdl3`). Then:

    cargo test
    cargo run --release -p harry

`ouro-harry [pack.zip] [level]` plays the game: main menu (New Game,
Continue Game, Instructions, Options, Credits, Quit), the story, the levels,
music and sound; a level number starts it at once. A data pack on the
command line, in `HARRY_DATA`, or as `ouro-harry-data.zip` next to the
program or in the current folder replaces the built-in one. Default keys
as on the Mac: ← → turn, ↓ scoot, ↑ inch, Space
kick, Control select item, Option (Alt) use item, 1–4 pick a weapon or the
swivel, Tab the map, Caps Lock pause; Command (or Ctrl) + I opens the
options during a game, F11 switches to full screen, Esc asks before giving
up the game. The game pauses when its window loses the focus. Keys,
scaling (whole pixels or free), full screen, volumes and saving between
levels are set in the options, which are saved in SDL's preference folder
(macOS: `~/Library/Application Support/MetalSnake/Ouro Harry`), with the
saved game. F8 / F7 jump to the next / previous level (for testing).
`ouro-harry --screenshot out.bmp [level] [frames]` runs the frames
without a window and writes the picture; with `HARRY_SAVE=<file>` it, and
Continue Game, start from that saved game.

The data pack is made from the original game, Harry the Handsome
Executive 1.0.1 (installed on a classic Mac or in SheepShaver, its files
as MacBinary in `re/work/installed/`), and the resource fork of Apple's
QuickTime Musical Instruments (`re/work/qtmi/`):

    cargo run --release -p harry-original --bin ouro-harry-extract -- \
        re/work/installed data/ouro-harry-data.zip

The tests against the original (crate `harry-original`) need the
recordings and files in the gitignored `re/work/` and are skipped without
them.

## Folders
- `crates/harry-core/`: game logic, one call per frame of the original, no
  I/O; the doc comments name the original's routines
- `crates/harry-data/`: the game data and the data pack the game reads
- `crates/harry-original/`: reads the original (MacBinary, resource fork,
  PICT, `cicn`, `HrSp` faces, `snd`, the levels, QuickTime music), writes
  the data pack; the tools `ouro-harry-extract` and `ouro-harry-repro`
  (saved games that put Harry where a bug of the original shows) and the
  tests against the original
- `crates/harry/`: the SDL3 frontend, `ouro-harry`
- `data/ouro-harry-data.zip`: the game data, built into the game
- `re/tools/`: the recorders that run the original's routines in Unicorn
  (`emu_*.py`), tools to run the original in systemless (`sl_repro.py`,
  `sl_peek.py`), the licence file for testing the original registered
  (`harry_license.py`) and the website's pictures (`site_assets.py`)
- `docs/`: the website (GitHub Pages)
- `NOTES.md`: how the original works, from the analysis of its code, and
  the port's deliberate differences ("Port improvements")
- `BUGS.md`: bugs and oddities of the original, how to see them, and what
  the port does with them
- `TODO.md`: what is still to do

## Credits
Harry the Handsome Executive © 1997 Ben Spees / Ambrosia Software, Inc.
Programming and design: Ben Spees; music: Eric Speier; plot and design:
Blake Wilson, Josh Rothman; game and title artwork: John Sledd; additional
artwork: Ben Spees; sound effects: Ben Spees (voice of Harry), Andrew
Welch / Ambrosia Labs, Juliet Wong. The instruments of the music that are
not the game's own come from QuickTime Musical Instruments © Apple. The
game data in `data/` is theirs; this project is not affiliated with them.

## License
The code is free software under the GNU General Public License, version 3
or later (see [LICENSE](LICENSE)). The game data in `data/` (pictures,
sounds, music, instruments, levels) is not covered by it: it belongs to the
authors of the original game and to Apple, as do the pictures of the
website made from it.
