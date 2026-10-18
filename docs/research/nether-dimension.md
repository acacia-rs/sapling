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
saw, ChangeDimension {1, (0.5, 70, 0.5) as given (feet), respawn false, loading_screen_id 0}, the
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
  draws; MT(0) would give 0, 3, 1, 0); no site: a 3x2 obsidian platform at y clamped to 70..height-10 with
  netherrack under the 10 air cells round it.

## sapling against the same drills

- `travel` (sapling-dims-travel.txt): the same packets in the same order and values; the ready
  PlayerAction comes later on the debug drill server, whose Nether is generated on the fly.
- `light`, `makes` (sapling-portal-light.txt, -makes.txt): the same portal cells and order, the same
  placeholder ChangeDimension, and the four Nether portals made at BDS's places and axes, the traveller
  put where BDS put it, yaw 90.
- `break`: the same cells emptied in the same order (the broken block as its own UpdateBlock).
- `survival`: the trip after 4.7 s against BDS's 4.04 (81 ticks): the same count, the debug server under
  a loaded testbox ticks slower.
- Seen only on sapling: after a flint and steel click sapling resends the clicked cells (UpdateBlock layers 0
  and 1), as it does for every block use.

## Left

- No drill made the no-site fallback (platform plus a netherrack ring round it, from the decompile only);
  spectators in portals, the End's dimension (2) and its
  platform, `/execute` beyond `in <dimension> run tp`, and BDS's per-tick UpdateSubchunkBlocks batching of
  other block changes.
- The empty-cell rule of the site search asks BDS's "can be replaced"; sapling uses a list of plants and fire.
- Sections of one made portal reach the client in BDS's order (upper section first in one capture),
  sapling's in cell order.
