# Bugs of the original

Every bug of the original, with what the port does (kept / fixed /
option) and why, and how to see it in the original.

## Seeing a bug in the original

Most bugs need Harry at a certain place with certain meters, ammunition
or items. `ouro-harry-repro` writes copier saves for them (in the
original's format, so Continue Game puts Harry right there):

    cargo run --release -p harry-original --bin ouro-harry-repro -- \
        re/work/harry-data.zip re/work/emu-ppc/shared/repro

In SheepShaver (the command is in NOTES.md, "Emulator") the folder is
on the `Unix` disk: Continue Game, open `repro`, pick the save. Each
save is also written as `<name>.hrsg` for the port:
`HARRY_SAVE=re/work/emu-ppc/shared/repro/<name>.hrsg`, then Continue
(or with `--screenshot` for a picture). Quicker than SheepShaver, without
a window: `python3 -I re/tools/sl_repro.py <name> [steps]` runs the
unchanged original, registered by a licence file, in systemless (NOTES.md, "systemless") - for looking; what
systemless draws wrong is listed there. The original's default keys:
← / → turn, ↑ inch, ↓ scoot, Space kick, Control select item, Option
use item, Tab map. A frame is 1/30 s. "Seen" below says whether the
bug was watched in the original yet; "expected" is what the code says.

## Candidates (to be confirmed, also in the 68k build)

- **Player direction 35 is lost on respawn.** `SetupPlayerSprite` restores
  `sp->dir = gPlayerDir % 35` (0x23); directions run 0–35, so 35 becomes
  0 (a 10° turn). Probably meant % 36. Port: keep (invisible in play),
  note.
  **See it:** save `dir35` (level 1, Harry saved facing direction 35).
  After Continue he faces direction 0 (left; 27 is up, ← adds one),
  one 10° step further left than saved - the same as a save made facing
  0. Not seen yet.
- **Death waits for the meter animation.** Death is tested on the
  displayed comfort / favor, which move 1 per frame toward the real value,
  so Harry dies up to 144 frames (4.8 s) after the real value reached 0
  and can be saved in that time. Port: keep (it is how the game plays).
  **See it:** save `death-delay` (level 11, comfort 19, standing on the
  invisible bridge between two holes). Turn and scoot off the bridge
  into a hole: the fall costs 20 comfort (level info #3), so the real
  value is −1 after the fall, but Harry lives on until the red bar has
  slid down to empty (19 frames, about 0.6 s) and only then dies. Not
  seen yet.

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
  **See it:** not visible as such - it changes which pixels of Harry's
  kick touch a wall (in each 8-pixel group the kick is tested
  mirrored). Only the Unicorn test shows it (`tests/masks.rs`).

- **Wall-shape masks indexed out of range.** `HitPlayerTileSprite`'s
  kick test looks up the mask of foreground kinds up to 54 in a table of
  32 wall shapes (`FUN_10054a14(160, …, 32, …)`), so kinds 32–54 read
  memory after the table. Unreachable in Episode 1 (foreground kinds
  32–55 are −1 there); a custom world with such kinds would test against
  garbage. Port: no mask for those kinds (no kick contact).
  **See it:** only in a custom world whose foreground kinds 32–54 are
  set (the level editor's tile kinds); not in Episode 1.

- **Half-pipe flip to the left tests the wrong axis.** `JumpScale`, during
  a half-pipe flip (air 200–299) with flip kind 0x26b0 (thrown left):
  `if (-0x200 < speedV) speedH = -0x200` - it tests the vertical speed
  but sets the horizontal one (the other three directions test and set
  the same axis). Confirmed by the differential test. Port: keep.
  **See it:** save `halfpipe-left` (level 14, right of a half pipe that
  runs up and down). Ride across it and let it throw Harry out to the
  left and, for comparison, to the right. Expected: to the right he keeps
  a fast speed out of the pipe (only raised to at least 2 px a frame);
  to the left his sideways speed is set to exactly 2 px a frame on every
  frame of the flip, however fast he was, so left flips are short and
  slow (while he is moving up fast, faster than 2 px a frame, it is not
  set at all). Not seen yet.
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
  **See it:** save `hole-scale` (level 11, on the invisible bridge
  between two holes). Kick sideways so that the jump ends over a hole:
  for one frame Harry is drawn with the scale −29 788 before he starts
  to shrink. What that frame looks like is the open question (TODO);
  step through with a screen recording at 30 fps or more
  (`re/tools/sl_repro.py hole-scale ... frames:120`). Scooting off the
  bridge does not show it (systemless, 2026-11-05: Harry shrinks from
  his normal size at once) - it needs a *jump* that ends over the hole.
  Not seen yet.

- **Harmless out-of-bounds read with no item selected.**
  `HandlePlayerSprite` checks `inventory[gCurInvSlot].id == -1`; with slot
  −1 it reads the word before the table, which is the idle-chatter timer
  (`_DAT_10065ac8`) - never −1, so nothing happens. Port: same read
  (`frame::handle_player`).
  **See it:** nothing to see (the value read is never −1).

- **The last strong B.A.D.A.S.S. shot backfires.** Releasing a strong
  shot (≥ 34 frames of charging) with 1–6 ammo: `HandleKeys` takes 6 ammo
  (clamped to 0) and sets shot kind 2; the next frame `MakeNewShot` sees
  no ammo and makes the out-of-ammo backfire (an explosion at the weapon,
  explosion and "empty" sounds) instead of the four shots. With 0 ammo the
  backfire is meant (the strong shot is still allowed, the weak one
  clicks empty). Probably the ammo was meant to be checked before taking
  it. Confirmed by reading both routines; both are differential-tested.
  Port: keep.
  **See it:** save `badass-backfire` (level 1, the B.A.D.A.S.S. selected,
  5 ammunition). Hold Option for 2 s (34 frames or more) and let go:
  instead of four shots there is an explosion at the gun with the
  "empty" sound, and the ammunition is 0. With 6 it backfires too (6
  taken, 0 left); with 7 or more the strong shot fires (save
  `badass-control`). **Seen** in systemless (2026-11-05,
  `re/tools/sl_repro.py badass-backfire option:150 frames:60`): with 5
  and 6 an explosion at the gun, Harry reddened and pushed back; with 7
  a shot and no explosion.
- **The mystery vial takes ammunition below 0 - for one pass.** Its
  outcome 5 subtracts 25 from all three counters without a lower bound,
  but `UpdateAmmo` clamps them to 0–100 later in the same pass of the
  game loop: on screen the effect is "below 25 → 0". Only code between
  the two in that pass (the collision pass, e.g. a magnet's "carries
  ammunition") sees the negative value. Port: the same (the clamp is
  ported).
  **See it:** save `vial-negative` (level 1, six vials selected, 10 of
  each ammunition, the staple gun carried). Each use (Option) draws an
  outcome; in systemless the fifth use was outcome 5 and all three
  counters read 0 afterwards (2026-11-05, read from memory through
  systemless's debugger: `re/tools/sl_peek.py`). The first use always
  gives the same outcome there (the clock is fixed); in an emulator,
  Continue the save again for another.

- **Shots test walls with a wrong rect.** `HitPlayerShotTileSprite` builds
  the shot's small test rect as (atV + 4, atH + 4, atV + 6, atV + 6): the
  right edge comes from atV (confirmed in the machine code at 0x100459b4).
  When the shot is right of the map's diagonal (h > v + 2) the rect is
  empty and walls of kinds ≤ 0x37 never stop it; left of it the rect
  reaches far to the right. Only then does `WallBounce` with the shot's
  mask decide. Fixing it changes 1 319 of the 22 500 recorded near-wall
  cases (shots that passed through stop, others pass). Port: keep (an
  option to fix it later).
  **See it:** save `shots-walls` (level 1's start room, the staple gun
  with 100 staples). The room runs from x 1920 to 2240; Harry stands at
  (2087, 2082), just right of the map's diagonal. Turn right and shoot
  at the right wall (a half tile, kind 2): the staples stay right of the
  diagonal, the rect is empty and only the shot's mask can stop them -
  expected: many pass into the wall. Turn left and shoot at the left
  wall: they cross to the left of the diagonal and stop at the wall.
  Not seen yet.
- **A stopped shot keeps hitting tiles.** It is only `frozen`; the
  remaining tiles of the same `CheckTileCollisions` pass are still
  handled, so its sound / soda explosion can come up to 4 times in one
  frame (seen in the recordings). Port: keep.
  **See it:** with save `shots-walls`, shoot into a corner where several
  wall tiles meet: the hit sound can come several times at once
  (louder, or doubled). Hard to tell by ear; the recordings show it.

- **Ammo bonuses add past 100 - for one pass.** `HitPlayerSprite` adds
  20 to an ammo counter while it is below 100, so it can reach 119 (gun
  pickups clamp to 100 themselves); `UpdateAmmo` clamps it to 100 in the
  same pass of the game loop. Not visible. Port: the same.
  **See it:** save `ammo-over-100` (level 1's start room, the staple gun
  with 99 staples). Five staple bonuses (711) lie up and right of the
  start, in the corridor at x 2180–2240, y 1950–1990: take one, the
  count is 100 (expected from the code and the vial's case; not read
  yet).

- **Kicking a locked door reads stale stack values.** `HitPlayerSprite`
  sets the door's hinge point only for 901, 911, 921, 931; for the locked
  903 / 913 / 923 / 933 the comparison reads two locals that were never
  set (machine code 0x1002a8bc ff.), so whether the kick counts as from
  the front or the back depends on whatever an earlier call left there.
  Port: uses the closed door's hinge (the recorder puts those values on
  the stack, so everything else is compared exactly).
  **See it:** save `locked-door` (level 1, in the corridor above the
  locked door 913). Kick it from the front, again after other kicks
  elsewhere: whether the kick counts as from the front or from the back
  can change between tries. What it changes on screen is not known;
  the stack values are only visible in Unicorn.
- **A kick that misses a door loses the kick's hot rect.** When Harry
  kicks a door he can neither open nor hit, `HitPlayerSprite` sets his
  normal hot rect for `RectSeparate` and does not restore the kick's
  (the restore tests an address against −8, never true). For the rest of
  the frame he is no longer kicking (e.g. in `CheckTileCollisions`).
  Port: keep.
  **See it:** not visible on its own (one frame without the kick's
  hot rect); only in the recordings.

- **A cancelled copier save still uses up the copier in the level.**
  `HitColliderSprite` sets the copier's level entry to 9199 (used) before
  `CopierSave` runs and leaves it there when the dialog is cancelled; only
  the sprite stays 9200 (and waits 300 frames). A level built again from
  its entries (a later save, a restored game) has the copier used.
  Port: keep.
  **See it:** save `copier-cancel` (level 6, left of the copier at
  (3864, 3579)). Touch the copier and choose Cancel. Go to the level's
  other copier (at (1803, 824), far up and left) and save there. Continue
  that save and come back: the first copier is used up (it does nothing)
  though it never saved. Not seen yet.
- **Wall guns other than 9401 / 9411 / 9421 / 9431 fire from garbage.**
  `HandleColliderSprite` takes every id 9401–9449 as a gun, but sets the
  shot's offset only for those four; the others use registers r27 / r28
  as the caller left them (and speed 0). None are in Episode 1. Port:
  offset 0.
  **See it:** only in a custom world with such guns; none in Episode 1.

- **Enemy shots hitting walls are silent.** `HitEnemyShotTileSprite`
  plays a sound only for "kind < 2001 and kind > 3000", which no kind
  is; the gun shot's own sound (251) is never heard either. Port: keep.
  **See it:** save `turret-shots` (level 7, near the turret 1901 at
  (633, 1541)). Let it fire and miss: its shots hit the walls without a
  sound (sound 251 is never heard). Not checked yet.
- **Homing shots steer once.** `HandleEnemyShotSprite` steers only on
  the frame `f7c` (12 at the start) reaches 1, so the "burst after 100
  steering frames" never comes; they fly straight after that until a
  wall, a shot or 35 slow frames. Port: keep.
  **See it:** save `homing` (level 8, near the scientist 1111 at
  (463, 1240)). Let the scientist throw a test tube and move sideways:
  it turns toward Harry once, shortly after the throw (12 frames), then
  flies straight on. Not seen yet.

- **Two pushed ramps meet in the wrong place.** `HitRampSprite` places
  the other ramp's hot rect at the other's centre instead of its `at`
  (so the obstacle is shifted by half a ramp down and right), and sets
  this ramp's hot rect to the other's kind's before bouncing. Port: keep.
  **See it:** level 14 has the four moving ramps of Episode 1 (9703 at
  (1804, 2061) and (1804, 2325), 9704 at (3071, 1914) and (3068,
  2406)). When two meet, they stop at the wrong place (half a ramp down
  and right). Whether two meet in play is not known; no save yet.

- **Faceless sprites make the tile pass read address 0.**
  `CheckTileCollisions` takes the centre from the face's bounds for every
  sprite with a tile task or +0x8b set, also when it has no face (the held
  weapon without a gun, a shot spawned this frame): it reads the bounds at
  address 0, low memory on the Mac. Harmless where only the foreground
  redraw uses it. Port: bounds (0, 0, 0, 0).
  **See it:** nothing to see (low memory is read, nothing changes).

- **Machine-gun bolts wrap their direction with 35.** `RandomMGShot`
  puts a bolt out along the robot's direction ± 2 and wraps it with 35
  instead of 36 (so 36 becomes 1 and −2 becomes 33), and then turns the
  flight direction by ±1 without any wrap: a robot facing 35 or 0 can fire
  a bolt with direction 36 or −1, which `AssignRobotShotSpeed` gives no
  speed of its own - it only drifts with the robot. Port: keep.
  **See it:** save `mg-bolts` (level 13, near the big robot 2001 at
  (1712, 1630)). Let it fire: when it faces direction 35 or 0 (left, or
  one step up from left), some bolts get no speed of their own and
  only drift along with the robot. Not seen yet.

- **Only one robot a frame bounces off walls.** The flags that keep a
  robot from bouncing twice off walls / obstacles are globals cleared by
  every robot's task, but the tile pass runs after all tasks: only the
  first robot in the list touching a wall or an obstacle that frame
  bounces, the others pass into it. Port: keep.
  **See it:** where two robots touch walls in the same frame (level 4
  has many), one of them goes into the wall. It happens by chance; no
  save.

- **A blob starts with a face from past its table.** `SetupBlobSprite`
  picks a direction 0–34 and sets the face to entry `dir` of a table of
  24 faces: for 24 and up it takes whatever lies behind the table (in the
  recorder the next loaded face or garbage). The blob's first frame sets
  a proper face, so it only matters if it is drawn before that. Port: no
  face for those.
  **See it:** nothing to see (its first frame sets a proper face before
  it is drawn).

- **A copier save does not name its world.** `DoCopierSaveGame` writes
  everything but the world file's name (+0x5518), which only
  `DoSaveGame` writes; `ContinueGame` opens a world other than Episode 1
  by that name. A copier save in a custom world before any other save
  of the session (with autosave off) keeps an empty or stale name and
  cannot be continued, or opens the wrong world. Port: keep for now (the
  port has only Episode 1); to fix with the fan worlds.
  **See it:** needs a custom world: start it, save at a copier with
  autosave off and no other save this session, then Continue that save
  - it does not open the world (or opens the one saved before).

- **The magnets' delay carries over.** The one counter for all magnets
  (`_DAT_1006c4ac`) starts at 2 when the game launches and is never
  reset, and every magnet in reach counts it down: where the first pull
  comes depends on the levels played before and on how many magnets
  reach Harry. Port: keep.
  **See it:** save `magnets` (level 11, the staple gun with 50 staples,
  above a narrow chute at x 1744–1776 that leads down between the two
  magnets at (1713, 1544) and (1802, 1542); they pull Harry down when he
  is above them and carries ammunition). Quit and start the original
  afresh, Continue, go down the chute and count the frames to the first
  pull; then Continue the save again without quitting and compare: the
  delay differs (the counter goes on from where it was). Not seen yet.

- **A meter's rounded end carries over to the next meter.** `UpdateMeters`
  sets its "3 px shorter, rounded end" offset (−3) once a meter's shown
  value is above 8 and never resets it within the call. A later meter
  drawn in the same call (comfort → surface resistance → corporate favor)
  is then cut and capped too, even at 8 or below: its strip ends 3 px
  short, and at a value under 3 the strip is gone and the end is drawn
  left of the meter, over its frame. The picture stays until that meter
  changes again. Port: kept (`Game::meters` records it as drawn).
  **See it:** save `meter-end` (level 1, comfort 72, corporate favor 2).
  After Continue the orange meter shows no strip at all where a 2-point
  strip with a rounded end belongs, and stays so until favor changes.
  (Surface resistance cannot be set this way: `HandlePlayerGlide` sets
  it from the floor every frame.) **Seen** in systemless (2026-11-05,
  `re/tools/sl_repro.py meter-end`): no orange strip, and the rounded
  end a speck left of the meter, over its frame - for as long as favor
  stays; the port shows the same.

- **A cheat balloon that does not fit reads a null sprite.** The
  P-A-R-G-H-Z and B-E-N balloons are `ShowBalloon(NULL, text, 2)`. When
  the balloon does not fit and the view pages (scroll mode 3), it nudges
  the view by the sprite's position - read through the null pointer, from
  low memory (0x2 / 0x4). Port: kept with that position as 0 (the view
  moves up and left), as the recorder's zeroed page 0 gives.
  **See it:** type P-A-R-G-H-Z when Harry is near the top or left edge
  of the window (the camera lags behind when he is fast - e.g. scoot
  up for a while): when the balloon does not fit, the view jumps up and
  left instead of toward Harry. Not seen yet.

- **Two instruments of Tune 614 have no name length.** Parts 2 and 5 of
  Tune 614 (level 14) ask for "Towerful" and "Racked" with the Str31's
  length byte 0 (the text behind it intact); their instrument numbers
  (0x8258, 0x8259) are the instrument component's numbering of the time
  the tune was made, not today's. What QuickTime then played is not
  known (by number: something else, by name: nothing). Port: the name
  read up to its first zero, so the instruments the tune meant play.
  **See it:** listen to level 14's music in the original (save
  `halfpipe-left`): whether parts 2 and 5 (Towerful, Racked) play, and
  with what. Needs QuickTime Musical Instruments in the Mac OS 9 system;
  not heard yet.

- **The registration check upper-cases one byte past the name.**
  `FUN_10058f9c` runs from length + 1 down to 1, so the byte after the
  licensee's name (inside the 0x100-byte field of the licence handle)
  is changed too if it is a-z. Harmless (that byte is not read).
  Port: no registration, not ported.
- **The registration check rewrites the licence file every time.**
  `FUN_10057cb0` reads "Harry License" and writes the same bytes back
  (`FUN_10058c94`, with FlushVol) on every call (seven callers: the
  menu line, level limits, ...). Harmless; on a locked disk the write
  fails silently. If the file cannot be opened at all (no Preferences
  folder, `FUN_10058ae0` returns 0), the check goes on with the nil
  handle and reads the name and code through address 0 - garbage
  compared, normally "unregistered". Port: not ported.

## Fixed in 1.0.1

Known from the 1.0.1 notes as fixed in 1.0.1 (so not
to be ported): music crashes, things happening twice in Reduce Frame Rate
mode, rolling boulders on some levels, "object of kind 0" dialog, missing
door in level 13.
