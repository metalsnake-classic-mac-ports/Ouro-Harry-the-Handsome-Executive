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

- **The last strong B.A.D.A.S.S. shot backfires.** Releasing a strong
  shot (≥ 34 frames of charging) with 1–6 ammo: `HandleKeys` takes 6 ammo
  (clamped to 0) and sets shot kind 2; the next frame `MakeNewShot` sees
  no ammo and makes the out-of-ammo backfire (an explosion at the weapon,
  explosion and "empty" sounds) instead of the four shots. With 0 ammo the
  backfire is meant (the strong shot is still allowed, the weak one
  clicks empty). Probably the ammo was meant to be checked before taking
  it. Confirmed by reading both routines; both are differential-tested.
  Port: keep.
- **The mystery vial can make ammo negative.** Its outcome 5 subtracts
  25 from all three ammo counters without a lower bound (firing clamps
  to 0). How the panel shows it is to be checked. Port: keep.

- **Shots test walls with a wrong rect.** `HitPlayerShotTileSprite` builds
  the shot's small test rect as (atV + 4, atH + 4, atV + 6, atV + 6): the
  right edge comes from atV (confirmed in the machine code at 0x100459b4).
  When the shot is right of the map's diagonal (h > v + 2) the rect is
  empty and walls of kinds ≤ 0x37 never stop it; left of it the rect
  reaches far to the right. Only then does `WallBounce` with the shot's
  mask decide. Fixing it changes 1 319 of the 22 500 recorded near-wall
  cases (shots that passed through stop, others pass). Port: keep (an
  option to fix it later).
- **A stopped shot keeps hitting tiles.** It is only `frozen`; the
  remaining tiles of the same `CheckTileCollisions` pass are still
  handled, so its sound / soda explosion can come up to 4 times in one
  frame (seen in the recordings). Port: keep.

## Fixed in 1.0.1

Known from the 1.0.1 notes as fixed in 1.0.1 (so not
to be ported): music crashes, things happening twice in Reduce Frame Rate
mode, rolling boulders on some levels, "object of kind 0" dialog, missing
door in level 13.
