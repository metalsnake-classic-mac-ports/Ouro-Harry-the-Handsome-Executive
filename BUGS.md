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
  (walls, Harry's body, obstacles) are not affected. Confirmed by running
  the original's `BuildMaskRegion` (with its `BlitEncFaceToPixMap` and
  `Threshold255Blit`) in Unicorn on all 929 faces, with `BitMapToRegion`
  reading bits the QuickDraw way: `harry_original::masks::from_face`
  matches every face only with the mirror. Pixels whose mirrored place
  falls beyond the face width are lost (e.g. the right 4 px of a 44 px
  face). Port: keep (`from_face` builds them the same way).

- **Wall-shape masks indexed out of range.** `HitPlayerTileSprite`'s
  kick test looks up the mask of foreground kinds up to 54 in a table of
  32 wall shapes (`FUN_10054a14(160, …, 32, …)`), so kinds 32–54 read
  memory after the table. Unreachable in Episode 1 (foreground kinds
  32–55 are −1 there); a custom world with such kinds would test against
  garbage. Port: no mask for those kinds (no kick contact).

- **Half-pipe flip to the left tests the wrong axis.** `JumpScale`, during
  a half-pipe flip (air 200–299) with flip kind 0x26b0 (thrown left):
  `if (-0x200 < speedV) speedH = -0x200` - it tests the vertical speed
  but sets the horizontal one (the other three directions test and set
  the same axis). Confirmed by the differential test. Port: keep.
- **One frame with a garbage scale when landing on a hole.** At air 999
  (set when a jump ends over a hole, background kind 7) no case of
  `JumpScale` sets the scale variable, so it uses register r30 as the
  caller left it: `HandlePlayerSprite` keeps the address 0x10068aa4
  there, so `scale0/1` = 0x8aa4 + 0x100 (−29 788) for that frame
  (recorded with r30 set like in the game). What this looks like on screen
  is to be checked (the drawing code scales by it). Port: keep the value
  (`air::UNSET_SCALE`), check the drawing.

- **Harmless out-of-bounds read with no item selected.**
  `HandlePlayerSprite` checks `inventory[gCurInvSlot].id == -1`; with slot
  −1 it reads the word before the table, which is the idle-chatter timer
  (`_DAT_10065ac8`) - never −1, so nothing happens. Port: same read
  (`frame::handle_player`).

## Fixed in 1.0.1

Known from the 1.0.1 notes as fixed in 1.0.1 (so not
to be ported): music crashes, things happening twice in Reduce Frame Rate
mode, rolling boulders on some levels, "object of kind 0" dialog, missing
door in level 13.
