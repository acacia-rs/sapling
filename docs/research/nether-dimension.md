# The Nether in the server: dimensions and portals

Branch `nether/dimension` (from `worldgen/nether` a45267e). How a world gets its Nether, how a player
changes dimension, and how Nether portals light, break and carry players, each as BDS 1.26.5x does it.
Evidence: drills of `crates/sapling/examples/bds_portal_probe.rs` (lines in `shared/portal_drill.rs`)
against the test BDS copy `~/work/bds-nether` on testbox (port 19186, flat world, seed 20261018), saved in
`research/captures/bds-dims-*.txt` and `bds-portal-*.txt`, the same drills against `drill_server`
(`sapling-*.txt`), and a bdsre reading of the BDS build behind `.testserver/bds-molang` (addresses below).

## Design

- **A dimension is a layer.** The Nether of a world is a second `sapling_world::World` of dimension 1
  (`World::void_in`, `World::with_storage_in(storage, 1)`, `WorldFolder::nether`, `Generated::nether`),
  added to the overworld's shard as its own layer (`Shard::add_dimension`, `ServerBuilder::nether`,
  `dimension_of`), named `<world>:nether`. It has its own storage keys (the LevelDB dimension prefix), I/O
  threads, load queue, generator and loaded columns; mobs, items and their saves follow the layer as for
  any world. Everything that was per layer (views, visibility, block changes, chunk sending) works
  unchanged. The layers of one overworld form a `Realm` (sapling-shard `dimensions.rs`).
- **One level.** Time, rules, weather and the portal records are the overworld's: the Nether's `Level` is a
  copy refreshed each tick (`dimensions::share_levels`); setting time or a rule in the Nether sets the
  overworld's and tells both. Only the overworld's world writes `level.dat` when it closes; the Nether's
  syncs the database (`WorldStorage::sync`).
- **The client's dimension** travels with the player (`ClientDimension`, with BDS's loading-screen counter
  from 0). A world move (`ctx.transfer`, a handoff) into a world of another dimension than the client shows
  is a dimension change; one in the same dimension stays the old swap.
- **Players** keep `DimensionId` and `Pos` (feet) in sapling's player record. One who left in the Nether
  joins there: StartGame waits (new `Stage::Packed`) until storage answered, so it names the Nether and the
  position (`dimensions::start_in`).
- **Portal records** are BDS's: the `portals` key (`data.PortalRecords`: `DimId`, `TpX/Y/Z` = the inside's
  lowest first cell, `Span`, `Xa`/`Za`), read and written with `level.dat` (`Level::portals`). BDS finds
  portals only by these, never by blocks.

## A dimension change (bds-dims-travel.txt, -silent.txt)

`/execute in nether run tp @s 0.5 70 0.5` on BDS, in one tick: RemoveEntity of every entity the player
saw, ChangeDimension {1, (0.5, 70, 0.5) as given (feet), respawn false, loading_screen_id 0} (one count for the whole run: two players' trips and a rejoin
went 0 to 7, bds-w1screens.txt; sapling's `LoadingScreens` in dimensions.rs), the
sleepers' LevelEventGeneric to every player; others in the overworld lose the player next tick, players of
the Nether get AddPlayer at once. Chunks of the new dimension follow (ChunkRadiusUpdate is not sent again,
nor SetTime, SetSpawnPosition, StopSound or PlayStatus). Once the columns near the player are sent BDS sends
PlayerAction {own id, DimensionChangeAck} (`playerSwitchDimension` 140863120), whether or not the client
said anything. A client answers with ServerboundLoadingScreen start at once and, on that PlayerAction, end
plus its own PlayerAction DimensionChangeAck; BDS then corrects it (CorrectPlayerMovePrediction, delta
-0.0784, on ground) to where it holds it: inputs from the loading screen are not taken. The next change
has loading screen 1. sapling does the same (`dimensions::change`, `acknowledge`; `chunks::send_chunks`
sends the PlayerAction with the join's spawn-radius rule and loads those columns first).

## Portal blocks (bds-portal-frames.txt, -light.txt, -break.txt)

- Frame: obsidian round an inside 2..21 wide and 3..21 high, corners not needed; inside may hold air,
  fire, portal; along x tried before z (`PortalShape::evaluate` 144788290, a port of Java 1.12's). Captured:
  1x2, 2x2, 22x3 and 3x22 do not light, 2x3, 21x21 and a corner-less frame along z do.
- Lit by flint and steel or a fire charge on a cell in the frame (no fire is placed: 141a79fc0), and by any
  fire placed there (fire's on-place handler; `/setblock fire` lit one). sapling: `fire::light` kindles
  first; other fire is noticed by the write hook and kindled in `blocks::portals::settle`.
- Shown as one UpdateSubchunkBlocks per section, flags 2, along the frame and each column from the top,
  then the ignite sound. `portal_axis` x for an inside along x.
- Breaking: a change beside a portal block re-evaluates the shape on its own axis; a broken frame or an
  inside not all portal empties the whole sheet at once (`neighborChanged` 144bb7630,
  `removePortalBlocks` 144788ca0), top row first, and drops its record. BDS sends the change and the
  emptied cells in one UpdateSubchunkBlocks; sapling sends the change as UpdateBlock, then the cells.

## Travel (bds-portal-light.txt, -survival.txt, -back.txt, -far.txt, -makes.txt)

- Timer (140d2e670, `PortalCooldownDurationComponent`): inside a portal with no cooldown the time in it
  counts up and the trip goes when it reaches the wait: 0 in creative (the next tick), 80 in survival and
  adventure (captured 4.04 s). Out of a portal the time decays by 4 a tick. Inside with a cooldown the
  cooldown is held at 10; a trip to or from the Nether starts it at 300. So the arrival portal never sends a
  player back until it stepped out (captured: 24 s standing in it).
- Scaling: x and z / 8 to the Nether, * 8 back, truncated toward zero; y unchanged (the eyes' height).
- Search: the record of the target dimension with a cell within 128 on x and z of the scaled place, nearest
  by whole-block distance with y (`_findPortal` 144786ac0). Found: ChangeDimension names that cell + 0.5;
  not found: the scaled place (BDS sends (0, -57.38, 0) for an overworld portal at x 4.5, z 5.5).
- Placement once the columns are loaded (`PortalForcer::force` 144783640): in the found portal at the same
  fraction across as the player stood in the entrance (entrance record within 128), on the bottom row,
  yaw turned 90 when the axes differ; MovePlayer mode Reset there, then the PlayerAction; no correction
  after the client's ack. A second overworld portal 160 blocks off linked to the first one's Nether portal.
- Making a portal where no record is near: Java's `makePortal` site search, ±16 on x and z, top down,
  a 3x4 footing then the frame alone, nearest to the arriving place; the frame 4x5 with the inside
  2x3, listed along the frame from the bottom; the first direction tried is `next & 3` of an MT19937
  seeded with the world seed's low 32 bits (four portals made in a row went 0, 2, 2, 0, as MT(20261018)
  draws; MT(0) would give 0, 3, 1, 0). Measured from the player's position, its eyes.
- Empty cell of the site search (new-version branch of `force`): not a liquid (Material `mLiquid`), no
  liquid in its extra layer, and `BlockType::canBeBuiltOver` (vtable slot 83, default 141a25540: the
  replaceable block component; no block type overrides it). sapling: `blocks::place::built_over`. BDS
  builds over leaf litter and over snow three layers high (bds-portal-voids.txt); footing is a solid material.
- No site (bds-portal-voids.txt, -collide.txt): direction 0, y = eyes clamped to 70..height-10; a 3x2
  obsidian platform at y-1 with air above it to y+2, then netherrack where the 10 cells round it under
  the floor are air, a row across at a time, both with flags 3; then the frame, flags 2.
- Section order: one UpdateSubchunkBlocks per section from `Dimension::mBlocksChangedBySubChunkMap`
  (an unordered_map: insert `onBlockChanged` 1418869d0, flushed and cleared in `Dimension::tick`
  14187f9c0), one cell alone as UpdateBlock. Captured, the section touched last goes first in six of seven
  made or lit portals across a boundary; the seventh (-collide.txt, x 1935) sent (121,4), (120,5), (120,4)
  for first touches (120,4), (121,4), (120,5). The map's list order with the decompiled hash fits neither,
  for any bucket count. sapling sends Nether portal sections last touched first.
  Read live 2026-11-05 (thirteenth2e, gdb on `Dimension::onBlockChanged`, worldgen-bds-debugger.md "Linux";
  `bds-portal-sections-gdb.txt` and `-hits.txt`): the captures are the Linux server's, whose map is libc++'s,
  not MSVC's. Its hash is boost's `hash_combine` of the sub-chunk x, y, z from seed 0 (emplace 0xc97d600);
  a key in an empty bucket goes to the front of the list, one in a used bucket just before that bucket's
  first. So sections go out first-touched-last unless two share a bucket. The x 60 frame (first touches
  (3,-3,2), (3,-4,2), (3,-3,1), (3,-4,1)) went out reversed, and -collide's x 1935 fits with 5 buckets, a
  fresh Nether map's count after its third insert (libc++ grows 2, 5, 11, 23 ...): (120,5) shares (120,4)'s
  bucket (hash both, mod 5). Within a section the cells go in touch order, and the
  frame's touch order is BDS's own (neither flood nor columns; the same in two runs).
  Ported 2026-11-07 (thirteenth3am, `blocks/send/hashed.rs`, `blocks/portals/order.rs`), fitted in
  scratch against every capture: the map is fresh each flush (no bucket history: bds-n13sheet.txt's first
  `/fill` of six sections fits only 0 to 5 buckets to start, after earlier ticks' changes). A lit frame's
  touch order is a libc++ `unordered_set<BlockPos>` (the same hash) begun with 64 buckets (so it grows
  128, 256, 512), filled along the width (an x frame west, x falling; a z frame south) with each column
  from the bottom up, and walked in its list order: all 441 cells of bds-portal-sections-gdb-hits.txt's
  x 60 frame in order, and the cells and sections of 27 of the 28 lit frames in the bds-portal-*.txt
  captures (the gdb run's x 156 frame sent its sections in another order, as another change in the
  same tick's map would make). The forcer's built portals (z frames in -makes, -voids) follow neither
  and keep sapling's order. Tests `a_big_frame_lights_in_bds_s_order`, `hashed.rs` unit tests. This is
  the Linux server's order; the Windows one (MSVC containers, its own `std::hash`) was never captured.

## `/execute in` (bds-t1dexec.txt)

- From the overworld, `execute in nether|the_end run setblock|testforblock|fill` answers
  `<command>.outOfWorld` (`generic.outOfWorld` for `fill`), then `execute.failed`: the other
  dimension's columns there are not loaded. `summon` answers `summon.failed`, `testfor`
  `noTargetMatch`. After `execute in nether run tp`, the same commands work in the Nether.
- `/structure` takes the dimension's heights: a 3-tall load at y 126 in the Nether is
  `structure.y.tooHigh ["125"]`, at -2 `tooLow ["0"]`; the End is 0..256. A save over columns not
  loaded is `generic.outOfWorld`; a load into them succeeds with `structure.load.queued` and is put
  once they load (sapling-shard structures/animate.rs).
- Commands above `gamedirectors` (`place`, `reload`, `permission`, ..., bds-t1denums.txt) answer
  `generic.error.permissions [name]`, then `execute.failed [name, sender]` (execute.rs).

## sapling against the same drills

- `travel` (sapling-dims-travel.txt): the same packets in the same order and values; the ready
  PlayerAction comes later on the debug drill server, whose Nether is generated on the fly.
- `light`, `makes` (sapling-portal-light.txt, -makes.txt): the same portal cells and order, the same
  placeholder ChangeDimension, and the four Nether portals made at BDS's places and axes, the traveller
  put where BDS put it, yaw 90.
- `break`: the same cells emptied in the same order (the broken block as its own UpdateBlock).
- `voids`, `collide` (sapling-portal-voids.txt, -collide.txt): the same portals, platforms and rings,
  the same made-portal packets, cells, flags and section order, but for the section order at x 1935
  (Left). As in BDS, the traveller is placed and the made portal is shown after its spawn columns.
  Before, sapling placed it earlier and sent the portal only inside the columns. sapling writes the
  portal before the columns go out, so a client that drops updates to sections it has not received
  still gets the portal.
- `survival`: the trip after 4.7 s against BDS's 4.04 (81 ticks): the same count, the debug server under
  a loaded testbox ticks slower.
- Seen only on sapling: after a flint and steel click sapling resends the clicked cells (UpdateBlock layers 0
  and 1), as it does for every block use.

- `execute in <dimension>` into a dimension nobody is in (bds-t1dexec.txt): BDS answered `setblock`,
  `testforblock` and `fill` with `outOfWorld`, a `structure load` with `load.queued` and a save with
  `outOfWorld`. Sapling's world there ticked every column (a headless shard runs without players), so
  they ran at once. Fixed 2026-11-04: ticking.rs `Region::ticks` ticks an empty world's kept columns
  alone while another world is peopled, and a column not yet loaded does not tick;
  `vanilla/structure.rs` queues a load and refuses a save there. Test
  `a_bot_goes_to_the_nether_by_command_and_back`; sapling-t1dexec.txt now answers as BDS's but for one
  `setblock` 3 s after the tp, `outOfWorld` on a Nether column the lagged debug drill server had not
  loaded yet (BDS: success), and the `testforblock` after it.

## Left

- Spectators in portals: drilled (bds-k1spec.txt, 2026-11-05): a spectator 8 s in a lit nether portal
  and 4 s in an end portal went nowhere; the same player in creative went through the nether portal.
  sapling skips spectators before both (`portals.rs` `tick`). BDS's per-tick UpdateSubchunkBlocks batching of
  other block changes is ported where drilled (checked 2026-11-05, twelfth wave): random ticks, flows
  and circuits each tick (`blocks::batched`), fills and plugin batches (`send/sections.rs`), pistons
  (flags 19, c8670476); captures still showing UpdateBlock on sapling where BDS batched
  (sapling-crops.txt, -concrete.txt, -bonemeal.txt, -conductors.txt) date from 2026-10-08 to 10-11,
  before the batches (3e338ec4, 2026-10-24). The map's order (above) is ported for lit and broken
  portals only: other batches (`send/sections.rs`) keep their fitted orders, which the map's gives too
  for two sections (last touched first) but not always for more (bds-n13sheet.txt's fills).
- Section order where it is not last-touched-first: settled, "Section order" above.
- A broken sheet (bds-n13sheet.txt, 2026-11-05: a 2x14 sheet over x 15-16 and y -26..-13, a side of the
  frame at (14, -20) set to air): BDS sent the frame cell inside the sheet's batch, not alone, and the four
  sections last touched first: (16,-32), (16,-16), (0,-16), then (0,-32) with the frame cell ahead of its
  column; cells top down. sapling-n13sheet.txt sends the frame cell first as UpdateBlock (the command's own
  write), then (0,-16), (16,-16), (0,-32), (16,-32). Fixed 2026-11-07 (thirteenth3am): a write that
  leaves a sheet beside it unframed is held (`portals/order.rs` `defer`) and goes first in the sheet's
  batch, sections in the map's order; test
  `a_broken_sheet_goes_out_by_sub_chunk_in_bds_s_order_with_the_frame_cell_first_in_its_own`. A frame
  corner set to air broke nothing on either.
