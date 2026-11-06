#!/bin/sh
# Rebuild the Ghidra project of the PowerPC build and decompile everything,
# with the shared pipeline (ouro-tools/re/ppc/run_ghidra.sh):
# names from the traceback tables and names.txt, TOC, TVectors, our globals
# (globals.txt) and the sprite record (sprite.txt) on the sprite routines.
# Usage: re/ghidra/run.sh            (from the repository root)
# Needs: re/work/app/pef.bin (data fork of "Harry 1.0.1"), Ghidra (brew),
# re/work/venv with capstone, ouro-tools (OURO_TOOLS, default ../ouro-tools
# next to this repository). Output in re/work/ppc/: game.pef, traceback.txt,
# toc.txt, functions.txt, toc_functions.txt, imports.txt, data.bin, game.c
# (see ouro-tools/re/ppc/run_ghidra.sh).
set -e
R="$(cd "$(dirname "$0")/../.." && pwd)"
W="$R/re/work"
TOOLS="${OURO_TOOLS:-$(cd "$R/.." && pwd)/ouro-tools}"
mkdir -p "$W/ppc"
cp "$R/re/ghidra/names.txt" "$W/ppc/names.txt"
PATTERN='(Setup|Handle|Hit|Init)\w*Sprite\w*|DoScoot|DoKick|DoInch|MoveChair|HandleKeys|'\
'HandlePlayerGlide|HandleGlide|HandleHalfPipe|ApplyModedImpulse|EnforceMaxSpeed|'\
'JumpScale|SpriteCenter|RectBounce\w*|WallBounce\w*|BGTileBounce\w*|HBounce|VBounce|'\
'GrateBounce|HandleRampCollision|HandleStairsCollision|ApproachMode|DoorSeparate|'\
'Find\w*Direction\w*|Accelerate\w*|Assign\w*ShotSpeed|Random\w*Shot|ThrowDart|'\
'MakeNewShot|FindHeldWeaponPos|HandleRepulsions|RobotTooFar|ShowBalloon'
PATH="$W/venv/bin:$PATH" sh "$TOOLS/re/ppc/run_ghidra.sh" "$W/app/pef.bin" "$W/ppc" \
    "$R/re/ghidra/globals.txt" "$R/re/ghidra/sprite.txt" 0xa2 Sprite "$PATTERN"
