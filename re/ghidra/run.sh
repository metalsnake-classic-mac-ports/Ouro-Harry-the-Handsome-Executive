#!/bin/sh
# Rebuild the Ghidra project of the PowerPC build and decompile everything,
# with the classic-mac-port skill's pipeline (scripts/ppc/run_ghidra.sh):
# names from the traceback tables and names.txt, TOC, TVectors, our globals
# (globals.txt) and the sprite record (sprite.txt) on the sprite routines.
# Usage: re/ghidra/run.sh            (from the repository root)
# Needs: re/work/app/pef.bin (data fork of "Harry 1.0.1"), Ghidra (brew),
# re/work/venv with capstone, the skill (CLASSIC_MAC_PORT, default
# ~/.claude/skills/classic-mac-port). Output in re/work/ppc/: game.pef,
# traceback.txt, toc.txt, functions.txt, toc_functions.txt, imports.txt,
# data.bin, game.c (see the skill's run_ghidra.sh).
set -e
R="$(cd "$(dirname "$0")/../.." && pwd)"
W="$R/re/work"
SKILL="${CLASSIC_MAC_PORT:-$HOME/.claude/skills/classic-mac-port}"
mkdir -p "$W/ppc"
cp "$R/re/ghidra/names.txt" "$W/ppc/names.txt"
PATTERN='(Setup|Handle|Hit|Init)\w*Sprite\w*|DoScoot|DoKick|DoInch|MoveChair|HandleKeys|'\
'HandlePlayerGlide|HandleGlide|HandleHalfPipe|ApplyModedImpulse|EnforceMaxSpeed|'\
'JumpScale|SpriteCenter|RectBounce\w*|WallBounce\w*|BGTileBounce\w*|HBounce|VBounce|'\
'GrateBounce|HandleRampCollision|HandleStairsCollision|ApproachMode|DoorSeparate|'\
'Find\w*Direction\w*|Accelerate\w*|Assign\w*ShotSpeed|Random\w*Shot|ThrowDart|'\
'MakeNewShot|FindHeldWeaponPos|HandleRepulsions|RobotTooFar|ShowBalloon'
PATH="$W/venv/bin:$PATH" sh "$SKILL/scripts/ppc/run_ghidra.sh" "$W/app/pef.bin" "$W/ppc" \
    "$R/re/ghidra/globals.txt" "$R/re/ghidra/sprite.txt" 0xa2 Sprite "$PATTERN"
