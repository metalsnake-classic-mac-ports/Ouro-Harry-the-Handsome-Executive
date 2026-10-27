# Bugs of the original

Every bug of the original, with what the port does (kept / fixed /
option) and why.

## Candidates (to be confirmed, also in the 68k build)

- **Player direction 35 is lost on respawn.** `SetupPlayerSprite` restores
  `sp->dir = gPlayerDir % 35` (0x23); directions run 0–35, so 35 becomes
  0 (a 10° turn). Probably meant % 36. Port: keep (invisible in play),
  note.
- **Death waits for the meter animation.** Death is tested on the
  displayed comfort / favor, which move 1 per frame toward the real value,
  so Harry dies up to 144 frames (4.8 s) after the real value reached 0
  and can be saved in that time. Port: keep (it is how the game plays).

- **Collision masks built from sprite faces are mirrored within each
  byte.** `Threshold255Blit` (used by `BuildMaskRegion`) packs the 8-bit
  pixels into the 1-bit mask least significant bit first (`slw r10, 1,
  x`, confirmed in the machine code), while QuickDraw bitmaps - and
  `BitMapToRegion` - read the most significant bit as the leftmost pixel.
  So every 8-pixel group of such a mask is reversed. Affects the regions
  of the player's kick poses (faces 107–143 of the face set, used by the
  pixel-exact kick test in `HitPlayerTileSprite`) and any other face set
  loaded with a region range. Masks from `cicn` resources and PICT 160
  (walls, Harry's body, obstacles) are not affected. Port: keep (build
  those regions the same way), to be confirmed in the emulator.

## Fixed in 1.0.1

Known from the 1.0.1 notes as fixed in 1.0.1 (so not
to be ported): music crashes, things happening twice in Reduce Frame Rate
mode, rolling boulders on some levels, "object of kind 0" dialog, missing
door in level 13.
