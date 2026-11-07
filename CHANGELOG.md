# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- The stapler's shot sound no longer jumps to a very low pitch now and
  then (its random rate above 65535 was cut to 16 bits).
- Sounds play where and as loud as in the original: each from the point
  the original gives it (stereo), the fan, the chair and some explosions
  quieter, a boulder's rumble from where it stopped.

## [0.1.0] - 2026-11-05

### Added

- First version: Harry the Handsome Executive 1.0.1 with all 15 levels of
  Episode 1, its menu, story, instructions, credits, music and sounds; the
  game data built in.
- Options of the port: keys, scaling (whole pixels or free), full screen,
  volumes, saving between levels; letters by the keyboard layout; a pause
  on focus loss; the options during a game (Command or Ctrl + I).
- Install with Homebrew: `brew install metalsnake-classic-mac-ports/games/ouro-harry`.

[unreleased]: https://github.com/metalsnake-classic-mac-ports/Ouro-Harry-the-Handsome-Executive/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/metalsnake-classic-mac-ports/Ouro-Harry-the-Handsome-Executive/releases/tag/v0.1.0
