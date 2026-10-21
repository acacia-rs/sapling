# The End in the server: dimension, end portals and gateways

Branch `end/dimension` (from `worldgen/end` bd12034). How a world gets its End and how players reach and leave it,
as BDS 1.26.5x does without the dragon fight. Evidence: drills of `crates/sapling/examples/bds_portal_probe.rs`
(the End's lines in `shared/end_drill.rs`) against testbox `~/work/bds-end` (port 19202, flat world, seed
20261018), saved as `research/captures/bds-end-*.txt`; the same drills against `drill_server`
(`sapling-end-*.txt`); a bdsre reading of the Windows build 6ab54bee (addresses below).

## Design

- **A third layer.** A world's End is a `sapling_world::World` of dimension 2 (`END`): y 0..255, unpainted columns
  `minecraft:the_end` (9), saved under the LevelDB keys with dimension 2. `Generated::end` (worldgen-bds-end.md),
  `WorldFolder::end`, `ServerBuilder::end` (`<world>:the_end`). It joins the overworld's `Realm` and shares its
  level as the Nether does (nether-dimension.md); `/execute in the_end run tp` and `ctx.travel` go there the same
  way. Columns ask for the sections up to the highest holding a block (BDS: 4 at chunk (6, 0)).
- **Blocks:** `blocks/end_portal.rs` (frames and the portal they make), called by the eye's use in `blocks/tools.rs`.
- **Travel:** `portals/end.rs` (end portal to the platform) and `portals/gateway.rs`, run from `portals::tick`;
  `dimensions::respawn_home` (the dead of the End and the Nether rise in their overworld), from `vitals/death.rs`.
- **Not built** (the dragon fight, left for later): the dragon, its boss bar, the exit portal, crystals' part in
  the fight, the egg, gateways made by kills, the credits.
- **The eye of ender thrown** (branch `end/eye`, below): `eyes.rs` in sapling-shard, from the ClickAir arm of
  `blocks/request.rs`; the stronghold from `World::locate_stronghold` → `WorldStorage::locate_stronghold` →
  `Generated` → `OverworldGenerator::locate_stronghold` (sapling-worldgen `structure/locate.rs`).

## By command (bds-end-travel.txt)

`execute in the_end run tp @s 100.5 49 0.5`: as the Nether's trip, ChangeDimension at the feet given, BDS's own
PlayerAction once the columns are sent, the correction after the client's ack. No platform: the actor stood
inside the island's end stone at (100, 49, 0) (a survival actor suffocates). `TeleportCommand` 143a15a10 calls
`requestPlayerChangeDimension` directly; only `DimensionTransitionPortalSystem` builds the platform. sapling the
same (sapling-end-travel.txt).

## Frames and the portal (bds-end-frame.txt)

- `EndPortalFrameBlock::use` 148a2ed10: an ender eye on a frame without one sets `end_portal_eye_bit`, flags 2,
  then `block.end_portal_frame.fill` at the middle of its top; outside creative the eye is spent. A frame holding
  an eye does nothing. No dimension check.
- The ring (148a30210 finds it from the clicked frame's facing, 148a31310 checks it): the twelve cells round a
  3x3, corners skipped, each a frame with its eye and `minecraft:cardinal_direction` facing the inside. A ring
  facing out did not light; one facing in did.
- Lit: one UpdateSubchunkBlocks, flags 2: the filled frame, then the nine `end_portal` cells x down then z down
  (the far corner first), then `block.end_portal.spawn` at the middle of the inside, a block up, then the fill
  sound. BDS gates it on a level bool (+0x62c) not identified; sapling always lights.
- BDS took a legacy `["direction"=N]` state on `/setblock end_portal_frame`; sapling's drills use the modern names.

## Through the portal (bds-end-frame.txt)

- `EndPortalBlock::entityInside` 148a2e480 (`InsideEndPortalBlockSystem`): a body touching an end portal goes at
  once, survival too (no 80-tick wait), from the overworld or the Nether, to the End; a sleeping or riding player
  does not. In the End it would be the exit portal's `changeDimensionWithCredits` (not built). The trip starts a
  portal cooldown of 12 BDS ticks; sapling keeps none (the platform is no portal).
- Sent at once: RemoveEntity of all seen, ChangeDimension (100.0, 50.0, 0.0) (the cell's corner, End
  `translatePosAcrossDimension` 1482797c0), the sleepers' count, CorrectPlayerMovePrediction there, delta 0.
- Once the columns round it are loaded (`PortalForcer::force` 144783640, End branch): MovePlayer Reset at
  (100, 50, 0), pitch 0, yaw kept, then the PlayerAction; the tick after, the platform: z from 2 down to -2, x 98..102,
  obsidian at y 49 and air at 50, 51, 52, flags 3, only the cells that differ (a second arrival sent two).
  No correction after the client's ack.
- sapling: the same packets and cells (sapling-end-frame.txt). Seen only on sapling: after the click it resends
  the clicked cell's two layers (BDS resends before); the platform's two sections go in the other order.

## Gateways (bds-end-gateway.txt)

The gateway worldgen put at (1953, 66, 181) for seed 20261018 (sapling-worldgen example `end_gateways`): a body
in it was teleported, in the tick after it got there, to (100.5, 51, 0.5): the exit (100, 50, 0) plus (0.5, 1,
0.5), no safe-spot search (block actor tick 1487fcae0), told as two MovePlayer Teleport (cause 0, source types 1
then 0) after a SetEntityMotion; the actor stood inside end stone there. The next tick the gateway was sent
again as UpdateBlock flags 19 (BDS also does that every ~10 s: the beam). The gateway then waits 20 ticks
(Java 40). Only in the End; ender pearls, riders and mounts are left. Block entity: `Age`, `ExitPortal` (three
ints), `EndGatewayBadPosChecked`; no `ExactTeleport`. sapling: a gateway's `ExitPortal`, else (100, 50, 0) (the
generated columns keep no block entity), the same two MovePlayer, the update the tick after (to the traveller too,
whose client still holds the column), the cooldown (sapling-end-gateway.txt); no SetEntityMotion and no periodic beam.

## Dying in the End (bds-end-death-none.txt)

The respawn state machine (1402beeb0) moves a player whose respawn dimension differs: ChangeDimension (0, (0,
32767, 0), loading screen none), the sleepers' count, the overworld's columns and PlayerAction at once, Respawn
searching at (0, 32767, 0), Respawn ready at the overworld spawn, EntityEvent Respawn, SetHealth 20, `reappeared`
heard at (0, 32767.1, 0). sapling the same (sapling-end-death-none.txt), Respawn ready in the same tick and its
usual MovePlayer after it; the Nether's dead go the same way. A drill answering the change with a loading-screen
end of id none was kicked (UnexpectedPacket): drill respawns with the `none` answer.

## The eye of ender (bds-eye-throw.txt, bds-eye-dims.txt, bds-eye-near.txt)

Drilled on testbox `~/work/bds-eye` (port 19210, seed 20261018, a **default** world: every other copy is flat,
and a flat world locates no stronghold); sapling's side on `drill_server` with `SAPLING_DRILL_WORLD=strongholds`
(void columns, the seed's locate), drills `eye-*` in `shared/eye_drill.rs` (`!throw yaw pitch`).

- `EnderEyeItem::use` 1436f6b50 (no `useOn`: a frame click only fills the frame, as before). It asks the
  dimension's generator for a stronghold from the floored eye position; none (Nether, End, flat) means nothing at
  all: no swing, no sound, the item kept (bds-eye-dims.txt). Else: EntityEvent ArmSwing, the eye spent outside
  creative (InventoryContent), AddEntity `minecraft:eye_of_ender_signal` at the eyes + (0.5, 0.5, 0.5) with no
  motion, then LevelSoundEvent `throw` (entity type `minecraft:player`) at the feet + 1.32 up and 0.15 along the
  yaw. BDS gates it on gamerule slot 40 (unidentified); sapling does not.
- The locate, `StrongholdFeature::getNearestGeneratedFeature` 1483745b0: the three spiral strongholds, then
  200-chunk cells in rings round the origin's (the first ring the whole 3x3; a cell holding a spiral start is
  skipped; a cell's chance is compared as a double, raw < 2^30), until a ring ends with a candidate; the nearest
  by squared chunk distance, the first on a tie. Answer (chunk x 16 + 2, sea level - 31 = 32, chunk z 16 + 2):
  BDS's `/locate` from the origin gave the same for seeds 20261003 and 20261018 (sapling-worldgen
  `structure::tests`).
- `signalTo` 142f9f750: further than 12 blocks across, the target is 12 blocks toward it and 8 up; else the
  stronghold block itself (bds-eye-near.txt: the eye settles round y 32). The 4-in-5 survival roll is the level's
  random on BDS, the shard's dice on sapling.
- Flight (`EyeOfEnderPreNormalTickSystem` 14469ee30), f32, from the throw's tick: move by the motion, then speed
  across += (distance - speed) x 0.0025, both x 0.8 within 1 block, rise += (+-1 - rise) x 0.015, aimed by Mth's
  table. The captured path matched to 7e-6 over its 80 told ticks; sapling's MoveEntityDelta and SetEntityMotion
  values equal BDS's bit for bit (sapling-eye-throw.txt). Every move is told, on_ground true, the last not.
- End (`normalTick` 142fa0450), on the 81st tick: kept, RemoveEntity and an `ender_eye` item tossed up at the
  eye (y + 0.125, 0.2 up); broken, LevelEvent 2003 (data 10) and 1041 at the eye, RemoveEntity the tick after.
- Left different: BDS pushes the eye out of its thrower's box (bds-eye-near.txt's fall through the player left
  the fit in x/z) and turns it (rot_x/rot_y of its deltas); sapling's item comes before the RemoveEntity, its
  stack ids do not count up, its item's toss is the shard's dice.

## Not done, and why

- **Spike bedrock and crystals.** `postProcessMobsAt` 148246230 puts an EnderCrystal (ShowBottom) at (x + 0.5,
  height + 1, z + 0.5) and bedrock at (x, height, z) for each spike whose centre is in the chunk, reading no fight
  state, but no caller was found, and in every drill (70 s with a player on the island) the tops stayed obsidian
  and no crystal was sent. Left out to match what BDS did.
- **Every arrival near the island starts BDS's dragon fight** (dragon spawn, boss bar, exit portal bedrock at
  0,61..65,0): the only differences in sapling-end-travel.txt (column (0, 0) asks for 5 sections on BDS, 4 on
  sapling; a tp to (0.5, 90, 0.5) lands on the exit portal's pillar at 66 on BDS, the island at 62 on sapling).
- **Leaving the End other than by death or command:** only the exit portal (the fight) leads back.
