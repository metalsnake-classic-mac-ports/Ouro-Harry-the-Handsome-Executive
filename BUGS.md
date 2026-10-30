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
  face). Port: keep (`from_face` builds them the same way). This also
  hits the wall masks of robots and reversors (cicn 598, loaded as the
  encoded face 4251) and blobs (face 1451): they bounce off mirrored
  walls, unlike Harry and the furniture (the cicn read directly).

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
  Barrels and chairs do not touch r30 at all, so they get what SATRun
  keeps there: the address of their own sprite record - a heap address,
  different on every Mac. Port: 0 (the scale 0x100 for that frame).
  Robots leave r30 as their own address or as a direction their code
  just computed, depending on the path. Port: 0 as well.

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

- **Ammo bonuses go past 100.** `HitPlayerSprite` adds 20 to an ammo
  counter while it is below 100, so it can reach 119; gun pickups clamp to
  100. How the panel shows more than 100 is to be checked. Port: keep.

- **Kicking a locked door reads stale stack values.** `HitPlayerSprite`
  sets the door's hinge point only for 901, 911, 921, 931; for the locked
  903 / 913 / 923 / 933 the comparison reads two locals that were never
  set (machine code 0x1002a8bc ff.), so whether the kick counts as from
  the front or the back depends on whatever an earlier call left there.
  Port: uses the closed door's hinge (the recorder puts those values on
  the stack, so everything else is compared exactly).
- **A kick that misses a door loses the kick's hot rect.** When Harry
  kicks a door he can neither open nor hit, `HitPlayerSprite` sets his
  normal hot rect for `RectSeparate` and does not restore the kick's
  (the restore tests an address against −8, never true). For the rest of
  the frame he is no longer kicking (e.g. in `CheckTileCollisions`).
  Port: keep.

- **A cancelled copier save still uses up the copier in the level.**
  `HitColliderSprite` sets the copier's level entry to 9199 (used) before
  `CopierSave` runs and leaves it there when the dialog is cancelled; only
  the sprite stays 9200 (and waits 300 frames). A level built again from
  its entries (a later save, a restored game) has the copier used.
  Port: keep.
- **Wall guns other than 9401 / 9411 / 9421 / 9431 fire from garbage.**
  `HandleColliderSprite` takes every id 9401–9449 as a gun, but sets the
  shot's offset only for those four; the others use registers r27 / r28
  as the caller left them (and speed 0). None are in Episode 1. Port:
  offset 0.

- **Enemy shots hitting walls are silent.** `HitEnemyShotTileSprite`
  plays a sound only for "kind < 2001 and kind > 3000", which no kind
  is; the gun shot's own sound (251) is never heard either. Port: keep.
- **Homing shots steer once.** `HandleEnemyShotSprite` steers only on
  the frame `f7c` (12 at the start) reaches 1, so the "burst after 100
  steering frames" never comes; they fly straight after that until a
  wall, a shot or 35 slow frames. Port: keep.

- **Two pushed ramps meet in the wrong place.** `HitRampSprite` places
  the other ramp's hot rect at the other's centre instead of its `at`
  (so the obstacle is shifted by half a ramp down and right), and sets
  this ramp's hot rect to the other's kind's before bouncing. Port: keep.

- **Faceless sprites make the tile pass read address 0.**
  `CheckTileCollisions` takes the centre from the face's bounds for every
  sprite with a tile task or +0x8b set, also when it has no face (the held
  weapon without a gun, a shot spawned this frame): it reads the bounds at
  address 0, low memory on the Mac. Harmless where only the foreground
  redraw uses it. Port: bounds (0, 0, 0, 0).

- **Machine-gun bolts wrap their direction with 35.** `RandomMGShot`
  puts a bolt out along the robot's direction ± 2 and wraps it with 35
  instead of 36 (so 36 becomes 1 and −2 becomes 33), and then turns the
  flight direction by ±1 without any wrap: a robot facing 35 or 0 can fire
  a bolt with direction 36 or −1, which `AssignRobotShotSpeed` gives no
  speed of its own - it only drifts with the robot. Port: keep.

- **Only one robot a frame bounces off walls.** The flags that keep a
  robot from bouncing twice off walls / obstacles are globals cleared by
  every robot's task, but the tile pass runs after all tasks: only the
  first robot in the list touching a wall or an obstacle that frame
  bounces, the others pass into it. Port: keep.

- **A blob starts with a face from past its table.** `SetupBlobSprite`
  picks a direction 0–34 and sets the face to entry `dir` of a table of
  24 faces: for 24 and up it takes whatever lies behind the table (in the
  recorder the next loaded face or garbage). The blob's first frame sets
  a proper face, so it only matters if it is drawn before that. Port: no
  face for those.

- **The magnets' delay carries over.** The one counter for all magnets
  (`_DAT_1006c4ac`) starts at 2 when the game launches and is never
  reset, and every magnet in reach counts it down: where the first pull
  comes depends on the levels played before and on how many magnets
  reach Harry. Port: keep.

## Fixed in 1.0.1

Known from the 1.0.1 notes as fixed in 1.0.1 (so not
to be ported): music crashes, things happening twice in Reduce Frame Rate
mode, rolling boulders on some levels, "object of kind 0" dialog, missing
door in level 13.
