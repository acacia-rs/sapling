# The End in the server: dimension, end portals and gateways

Branch `end/dimension` (from `worldgen/end` bd12034). How a world gets its End and how players reach and leave it,
as BDS 1.26.5x does, and the dragon fight (its own section). Evidence: drills of `crates/sapling/examples/bds_portal_probe.rs`
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
- **The dragon fight** (branch `end/dragon`, section below): sapling-shard `dragon.rs` and `dragon/`, the way
  home in `portals/exit.rs`, the record in sapling-leveldb `dragon_fight.rs`.
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

## The dragon fight (bds-dragon-*.txt)

Drilled with `bds_dragon_probe` (drills in `shared/dragon_drill.rs`) on testbox `~/work/bds-dragon` (port 19220,
default world, seed 20261018, easy), sapling's side on `drill_server` (`sapling-dragon-*.txt`). Readings:
research/dragon-fight-decompile.md (the fight manager) and research/dragon-ai-decompile.md (the dragon).

- **Start** (watch, fight): a player in the cube ±200 round the origin with columns (-1|0, -1|0) loaded makes
  the dragon at (0, 128, 0), random pitch, the spawn sound at its head, its data told twice; the unlit podium
  (3D pattern, torches) a tick later; ShowBar about every second to players within 125. No spike crystals or
  bedrock in the first fight. sapling the same; it resends the podium's columns to clients still waiting on
  blobs, since its fight starts while they stream in (an again run lost half the podium on the bot without it).
- **Flight** (watch): BDS's goals (holding pattern, strafe, charge, landing, scanning, flaming, takeoff, death)
  over the 24 nodes (rerun after the first-growl fix: perches 8 vs 8, first growl 5.0 s after spawn on both). Two
  moves a tick: the generic move by the stored motion, then **xz × 0.91** (fitted:
  without it sapling flew at twice BDS's speed), then the goal's steer, move and drag. The first told moves
  are equal to 4 digits (same first node and yaw 357.19); over 75 s median speed 0.869 vs 0.877, flaps 89 vs
  85, growls 4 vs 4, the first 100 ticks after spawn, 8 vs 6 perches. A survival player near the portal is
  strafed and charged; landing needs none (creative players are no prey).
- **Hurt time** (hits): a second hit 6 ticks after one takes nothing on BDS and sapling (the 10-tick guard,
  run down in the dragon's own tick, since the creatures' tick passes it by); its health goes out as
  UpdateAttributes as BDS's (198.75, 197.5); while guarded its wings and head hurt no one (AI doc 3 step 10).
- **Fireball turning**: pitch and yaw go a fifth of the way to its motion's each tick and are told
  (MoveEntityDelta rot_x/rot_y, head 0), as BDS's turned from -14.06/-5.62 to 347.34/352.97.
- **Hits** (hits): a fist's 1 takes 1.25 (`min(d,1)+d/4`); only players and block explosions hurt it; a lethal
  hit leaves 1 and sends it to the portal to die; sitting, a quarter of its health lost makes it take off.
- **Death** (fight): DeathAnimation, flags reset, the death sound 2 up, 200 ticks, then the portal lit, the egg on
  the first kill, a gateway in BDS's MT-shuffled order of 20 (first (0, 75, -96)), experience 12000 then 500.
- **Gateway exits** (again, exits; research/dragon-extras-decompile.md 1-3): the walk out 1024 along the gateway's
  direction tests the **column** at the point, not Java's chunk (BDS walked past a chunk holding one end stone);
  then BDS's `findValidSpawn` (column tops within ±16, end stone with no air round it first, nearest in 3D) and
  `findTallestBlock` (chorus counts), the far gateway 10 over the tallest block within 16 of the spawn, and the
  landing one over the spawn search round the far gateway: BDS reads that through the near side's view, so its
  tallest-block step falls back to the spawn. Both kills' exits equal BDS's: (-13, 76, -1123), (585, 71, -850).
  Where no spawn is found BDS builds an end island at (x, 75, z) of the walk's point (drills `exit-island`,
  `exit-island2`: the first gateway's line and the second's exit cleared with `/fill`). BDS's walk still
  stopped at the column the island had stood on, so its emptiness test does not see a `/fill` (a stale
  heightmap, presumably); its spawn search did, and it built the island there. Both islands are Java's
  discs going down from 75 (layers of 145, 97, 45, 37, 9 cells and 6 layers from radius 6), the far gateway
  10 over their first top column and the exit one over it, as sapling builds them. The island's random is a
  fresh MT19937 seeded by hash_combine of the cell's `std::hash<int>`s (148274c80; seen live on a Windows BDS
  attached mid-fight: seed 0xF92C2FAE at (0, 75, -1120), MSVC's FNV-1a over each coordinate's bytes).
  sapling builds worlds as Windows BDS does (the user's rule, as all of worldgen), so it seeds the island so:
  radii 5, 4.5, 3, 1.5, 1 there. The testbox's Linux BDS hashes an int as itself (libstdc++) and built
  other islands (6, 4.5, 3, 2.5, 1 and 6, 5.5, 4, 3.5, 2, 1.5 in the two drills); a unit test keeps that
  seeding reproducing them, so testbox drills of islands differ from sapling by design.
- **Fireball** (survival, survival2; extras 4): the strafe shoots from two over the dragon's feet, a block towards
  its prey, at the middle of the prey's box, at 1.3 with a 0.075 gaussian spread; AddEntity (owner, flags, box
  0.31) and its data, the `disappeared`/`undefined`/`shoot` sounds, its first move in the same tick, flags the next,
  `reappeared` the one after; straight flight told every tick; on a block or player box grown by 0.3 a splash
  (LevelEvent, its colour) with no move told, and the tick after the cloud at the cell's middle, the fireball gone,
  `explode`. A strafe starts, and aims, only at a player the dragon sees (`canSee` 140d4f980: no block with a box
  between its head and the player's eyes; BDS's attach points 3, the player's taken as its eyes), and the scan's
  takeoff target must be seen too. The fight's draws are unrepeatable, as BDS's level random, so the rate is a
  distribution: fireballs per 150 s survival drill (survival, survival2..6): BDS 1, 4, 3, 5, 6, 5 (mean 4.0);
  sapling 3, 3, 5, 2, 5, 5 (mean 3.8); before the sight test sapling shot 6-7. Flames (pukes) 32 in most runs of both.
- **Fireball, more** (extras 5.1-5.3): a mob's box pushes what overlaps it as Java's `Entity.push` (the larger
  axis by 0.05), so the fireball gains speed away from the dragon's feet while in its 13-wide box, about 4
  ticks; fitted on bds-dragon-survival.txt's first fireball with the push between the dragon's stored-motion
  move and its own, and sapling's fireballs now gain the same 0.05 a tick. It meets mobs as players (not
  dragons); a hit mob gets only the cloud. A player's melee does nothing to it on BDS (its `_hurt` is empty).
- **Clouds harm mobs** as players (half an instant harm; the undead mended). BDS's pulse, which lists mobs
  before players and stops at its owner, should spare every player while the sitting dragon is in the box
  (extras 5.4), but bds-dragon-survival.txt's flaming cloud hurt the player three times with the dragon
  sitting, so that rule is not copied.
- **Start**: BDS builds the dragon's nodes on its first tick whatever is loaded and once more when the 8 x 8
  columns round the middle are, then keeps them (extras 5.5, correcting the AI doc). sapling does the same,
  and tells the dragon's first move (it used to wait for its columns, ~1.6 s, and skip that move). It also
  asks for those columns: a sapling player's view never loaded them all, and nodes kept from unloaded
  ground (all at 73) cut the fireballs to 0, 0, 2, 0, 2, 0, 2 a run; asked for, 2, 3, 6 against BDS's mean 4.
- **Wing and bite** hurt on the world's difficulty (BDS's easy: 3.5 and 6) and throw 0.4 away and up as a hit.
- **Breath clouds** (survival): the fireball's radius 6 for 2400 ticks, the flaming dragon's radius 5 for 200 at
  the tenth breath tick, half the radius out from its head on the ground; told with BDS's metadata; every fifth
  tick of age, players within the radius take 3 by magic (half an instant harm) and the throw of a magic hit; the
  fireball's spares one for 20 ticks (BDS and sapling both hit once a second there).
- **Reach**: a hit reaches the dragon by its head, neck, body, wings and three tail parts (BDS's 64-tick trail;
  its tail reads 4, 6 and 8 back). Its target (TargetEid) is told as a strafe or a scan takes and drops one.
- **Summoning again** (again): four crystals on the portal's sides; growls on the same ticks as BDS (34, all
  within half a tick), pillars rebuilt one per 40 ticks, the new dragon at tick 604.7 vs 604.8, the four
  crystals blown up, another gateway on its kill.
- **Home** (home): ShowCredits once per player; ChangeDimension respawn, Respawn searching then ready, SetHealth,
  MovePlayer Reset; the second time no credits.
- **Kept** (resume): `TheEnd` encoded as BDS's bytes; a kept killed fight makes no dragon (shard test).
- **API**: `ShardCtx::dragon_fight`, `DragonKilledEvent`, `DragonRespawnEvent`; the dragon is an entity.

## Not done, and why

- **Dragon:** another projectile does not send its fireball back (BDS: along the shooter's look at speed 1,
  extras 5.1); the cloud's owner stop is not copied.
- **The second kill's orbs**: sapling's did not join up (77 entities against BDS's 24); with merging built they
  list as BDS's (26, 26, 43 against 24, 24, 46; docs/research/experience.md "Drilled").
- **Spike bedrock and crystals.** `postProcessMobsAt` 148246230 puts an EnderCrystal (ShowBottom) at (x + 0.5,
  height + 1, z + 0.5) and bedrock at (x, height, z) for each spike whose centre is in the chunk, reading no fight
  state, but no caller was found, and in every drill (70 s with a player on the island) the tops stayed obsidian
  and no crystal was sent. Left out to match what BDS did.
- **Column (0, 0)** asks for 5 sections on BDS, 4 on sapling (sapling-end-travel.txt, before the fight was built).
