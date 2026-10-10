# BDS liquid flow at decoration (crates/sapling-worldgen/src/feature/liquid, springs.rs)

Companion to [worldgen-bds.md](worldgen-bds.md). The flow rule itself (`LiquidBlock::_tick` 148b5d060 and helpers) is
[water.md](water.md)'s; this is when decoration runs it and on what.

## Where ticks come from
- **Aquifer fluid** (`Aquifer::computeSubstance` 1481dfe30): +0x25, Java's `shouldScheduleFluidUpdate`, is set when the two
  nearest aquifer centres have `1 - |d1 - d2| / 25 > -0.4` (below y -54, only at y -55). `_prepareHeights` (14822eb60)
  and the carvers (`carveBlocks` 148283e70, canyon 148280880) hand each fluid cell they place with the flag to
  loadChunk's lambda 148263880, which queues a tick in 1 on the chunk's own `BlockTickingQueue` (LevelChunk +0x130).
- **Springs**: `legacy:springs_feature` (rule `overworld_surface_springs_feature`, surface pass, place 149e97230): 25
  water tries `z = nextInt(16)`, `y = nextInt(256) - 64`, `x = nextInt(16)`, then 20 lava tries with
  `y = nextInt(nextInt(nextInt(240))) - 56`, skipped in deep dark and in biomes tagged frozen + ocean or
  ice_plains + mutated. `SpringFeature::place` (149d65900): rock above and below, air or rock at the cell, rock on
  exactly three sides (with below: 4) and air on the fourth; rock (149d65e10) = stone, deepslate, tuff, calcite,
  snow, powder snow, packed ice, dirt, coarse dirt (no granite, diorite, andesite). Sets the source with flags 3,
  ticks it in 1. 213 of 215 springs near origin match.
- **Feature writes**: `updateNeighborsAt` (1415f89a0: x-1, x+1, z+1, z-1, y+1, y-1) calls `neighborChanged` on
  liquids beside, which queues a tick unless one is due at the same time (148b5cd30 scans the pending heap).
  `setBlock` does not call it in worldgen (Neighbour updates, below); `Feature::_setBlock` (144c0cc40) records the
  write in the feature's transaction when it has one, and the commit (1415483f0/141548760: `onPlace`, ticks,
  `updateNeighborsAt` per block) does. sapling updates at each feature write, which the oracle preferred to no
  updates; which hardcoded features run without a transaction is not read. The freeze pass writes with flags 2.
- **Border water**: after the freeze pass, in the chunk's 60 border columns (table 14a923c90) from y -64 up to the
  light heightmap, the lowest cell of each run of layer-0 water is set to a source (flags 0) and ticked in 1.
  The heightmap is frozen while a chunk is post-processed (`LevelChunk::setBlock`'s upkeep 1409f3d60 returns
  unless the chunk's state +0xf0 is 11; rebuilt by 1409e29f0 at the end of loadChunk and after decoration, task
  lambda 140a16f40; the same finding as worldgen/trees3's for `query.heightmap`): water that flowed in above the
  column's frozen top is never reset.
- **Not structure pieces**: `StructurePiece::placeBlock` (141f59f90, `setBlock` flags 4) and template placement
  (14160c130) queue a tick in 1 for a liquid they place, behind `!level.vt+0x9e8() || source.mAllowTickingChanges`
  (+0x58). The oracle says the gate is closed: pieces' `water[liquid_depth=8]` cells (trial chambers, ocean ruins)
  stay 8, and ticking them cost 11 cells and gained none.

## How they run (end of `decorationPostProcessChunk` 1482143a0)
- The block source decoration uses carries two local `BlockTickingQueue`s (+0xe0/+0xd8); every tick queued while
  decorating lands there. After the border scan the chunk's own queue is moved in (`acquireAllTicks` 141607780: pop
  its heap, push each).
- Then up to 128 `tickPendingTicks(until = u64::MAX, max = INT_MAX, instatick)` (1416042f0) calls while ticks remain.
  Each call sets the clock to `u64::MAX`, moves every pending tick to an active heap and runs them; ticks queued
  meanwhile land at `MAX + delay` (wrapped: water 4, lava 29) and wait for the next call, so a call is one BFS
  generation. Heaps are MSVC `push_heap`/`pop_heap` on (time, priority); their order among equal keys decides which
  cell flows first, so sapling replicates them (ticks.rs).
- `BlockSource::addToTickingQueue` (1415f50f0) takes a tick into that local queue only when every chunk within 8
  blocks of the cell is in the block source (the 3x3); otherwise the tick goes to the cell's own chunk queue
  (141603fd0) and waits for the game. Springs, border water, spreads, depth changes and neighbour updates all pass
  through it; loadChunk's aquifer ticks and the requeue below do not. So decoration flows liquid only in the
  32x32 blocks centred on the decorated chunk.
- An instatick needs every chunk within a block of the cell in the 3x3; otherwise it is requeued with delay 0 (time
  `MAX`, never due in game). Leftovers go back to the chunk's queue; in game they run at the next ticks.
- sapling outputs decoration's result; the chunk-queue ticks can be played afterwards as game ticks on the source's
  3x3 (`SAPLING_LIQUID_GAME_TICKS`, default 0). TODO: hand them to the server's own ticking instead.
- While instaticking, `neighborChanged` reaches only blocks with BlockLegacy +0x163 bit 3, which `LiquidBlock`'s
  constructor (148b5c4c0) sets: liquids do hear their neighbours. `spread_to` replaces whatever it enters without
  drops or waterlogging.
- **Neighbour updates**: `BlockSource::_blockChanged` (1415f6fd0) updates neighbours only when
  `shouldFireEvents(chunk)` (vtable +0x98, 1415f5aa0) holds: the source does not allow unpopulated chunks and the
  chunk's state (+0xf0) is 11. In worldgen it never does, so a flags-3 write wakes nothing by itself. The liquid
  code calls `updateNeighborsAt` where it wants it: `_tick` after a depth change (148b5d8b4) and `_trySpreadTo`
  when instaticking (148b5e805). A cell drying to air, lava hardening and the stone under falling lava do not:
  lava that lands on water makes one stone and stops, a cut-off stream keeps its last cells.
- **Lava** asks the block's `isLavaBlocking` (vtable +0xc8) where water tests faces. The base (141a28450) is false
  for `mThickness > 0`, else the material blocks motion or is solid; azalea, bamboo, rails, dripleaf, ladders,
  multiface blocks (glow lichen, sculk vein), snow layers, lily pads, candles, campfires and scaffolding override it
  (LeviLamina headers). sapling treats those as holding lava (badlands: lava beside a sculk vein, vein intact);
  coral plants and fans override it too but the oracle read better without them, and which blocks have a
  thickness is not read.

## Game time (2026-10-10, worldgen/res6)
What the earlier sections call "flows BDS ran in game" is `LevelChunk::tick` (1409f98a0). While `LevelChunk`
+0x10dc is 0 and the chunks around are loaded, each game tick calls `tickPendingTicks(until = current tick, max
100, instatick = 1)` on the chunk's block queue (+0x130) and random queue (+0x138); the flag is set once both
return false or after 200 such ticks (counter +0x10d8). After that the same two calls run with instatick 0. So a
chunk's leftover decoration ticks are flushed over its first ticks in a ticking area, at game speed.

`scripts/bds_still.py <exe>` patches a BDS copy so those four calls do nothing (decoration's own calls stay);
`tools/worldgen-oracle/run.sh` with `WORLDGEN_BDS` on that copy saves worlds that hold worldgen only:
`20261003-still` and `-still-run2` (the near-origin and far-out regions; test `blocks_match_bds_still`).
What the still world showed:
- **Inherited ticks.** Most of the "game time" water was decoration's. A tick `addToTickingQueue` routes to its
  cell's chunk queue stays there, and `acquireAllTicks` at that chunk's own decoration takes the whole queue, so
  a flow an earlier source could not finish continues when the cell's chunk is decorated later. sapling dropped
  those ticks; `ChunkWrites::routed_to` now hands them on (`liquid::settle`'s `inherited`, after the chunk's
  aquifer ticks, earlier sources in decoration order). Ticks routed to a chunk decorated earlier wait for the game.
- **Bubble columns.** Decoration leaves one bubble column cell over a magma block (250 cells, one per column);
  the column above grows by game ticks (1709 cells in the ticked world). `blob.rs` places the one cell, the
  whole column only under a `SAPLING_LIQUID_GAME_TICKS` replay; the judge files BDS bubble columns over our
  water as "game time" in the ticked worlds.

Liquid errors (`scripts/liquid_residue.py`) and deterministic cells against the still world, before and after
the two changes:

| region | liquid errors | wrong | missing | deterministic |
|---|---|---|---|---|
| near origin (still) | 1019 -> 292 | 2829 -> 572 | 233 -> 206 | 2788 -> 504 |
| far out (still) | 15 -> 1 | 260 -> 129 | 20 -> 20 | 251 -> 120 |

256 of the 292 lie within 4 blocks of a lichen or sculk difference. The other four worlds have no still twin
yet: their liquid residue still holds the flush.

## Oracle (2026-10-07)
Fluid cells differing near origin went 12.0k -> 1.7k. The 8-block routing removed most of our over-flow (infinite
sources BDS's water never formed, lava floods into obsidian: water-over-air cells 383 -> 71, far out 218 -> 108
Liquids errors, negative seed 42 -> 3). What remains near origin is about 1.3k cells of BDS water where ours is air:
flows BDS ran in game from chunk queues while the oracle's tiles were ticking. How long a world ticked depends on the
oracle run (a long game phase fixes near origin and breaks the small worlds), so it is not modelled.
Not modelled either: lake and monster-room writes' updates, lava's 3-in-4 recede delay (TLS random), waterlogged
plants losing water; about 100 lava cells near origin still spread where BDS's did not (a falling lava cell that
made stone below then spreads sideways in ours): explained on 2026-10-09 (Neighbour updates).
In biome regions the liquid residue of old growth pine taiga (lush caves) and of the cave regions sits where
cave features still differ ([worldgen-bds-caves.md](worldgen-bds-caves.md)) and changed the walls a spring or a
flow needed. Bubble columns over magma grow by ticks: near a source's 8-block edge they wait for game time.

## Oracle (2026-10-09, worldgen/liq2)
Liquid errors per world, before and after the neighbour-update rule, lava blocking, the frozen heightmap (the
trees3 commits, carried here) and wet pointed dripstone:

| world | liquid errors | fixed by a 600-tick replay | left: thread-local / feature / structure / rest |
|---|---|---|---|
| near origin | 1731 → 1623 | 968 | 407 / 30 / 0 / 218 |
| far out | 109 → 101 | 92 | 7 / 0 / 0 / 2 |
| negative seed | 3 → 3 | 1 | 2 / 0 / 0 / 0 |
| biome regions | 933 → 676 | 155 | 323 / 46 / 33 / 119 |
| tree regions | 884 → 848 | 278 | 116 / 389 / 9 / 56 |
| structures | 2206 → 2093 | 781 | 890 / 197 / 20 / 205 |

`SAPLING_ORACLE_DUMP` now holds the liquid errors (stage L) and fluid-against-solid cells (W);
`scripts/liquid_residue.py <dump> [--compare <dump with SAPLING_LIQUID_GAME_TICKS>]` splits each flow
(26-connected cells) by the first cause any of its cells has: a lichen or sculk difference within 4 blocks, another
stable error within 2 (a feature block, a spring's rock cell), a BDS structure block near, or none.
- **Game time is the floor and one tick count does not close it.** The worlds' `currentTick` at save is 601 (near
  origin), 217 (negative seed), 822 (biomes), 1305 (trees), 1426 (structures), but a chunk ticks only while its
  tile's ticking area lives. Replaying 600 ticks fixes the cells of the third column and breaks others (flows BDS
  never ran, or ran with neighbours our 3x3 lacks): liquid errors in total 5344 → 5699, near origin 1623 → 1359,
  structures 2093 → 2717, far out 101 → 212, negative seed 3 → 42. Keep the default at 0.
- The stronghold region (760 → 700) is mostly one flow: a spring near -1472,46,-1896 whose water runs down a cave
  to y 7 (about 570 cells). Ours stops near x -1465, where cells come within 8 blocks of the source's 3x3 and
  their ticks go to the chunk queues; BDS ran the rest in game. 401 of the 700 come back with the replay, the
  others need chunks past the source's 3x3. No tick routing at the pieces is involved: fewer than ten wrong
  water cells lie within 6 blocks of a stronghold block.
- What the "left" columns hold: flows steered by a lichen, sculk or other feature difference (in old growth spruce
  taiga a vegetation patch that differs from BDS's covers the open side of a lava spring at -8533,32,-458, and
  over 150 lava cells below are missing), flows entering from ring chunks whose sources the oracle did not save,
  and waterlogged plants.
- Fixed here: desert (127 cells) and the stronghold region (60), where a border reset turned falling water above
  the generated top into sources that then multiplied; near origin's lava patches over lakes (about 70 lava and
  50 stone cells): the 2026-10-07 "unexplained" sideways spread after a fall made stone.
