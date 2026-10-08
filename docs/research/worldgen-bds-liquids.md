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
- **Feature writes**: features set blocks with flags 3; `updateNeighborsAt` (1415f89a0: x-1, x+1, z+1, z-1, y+1, y-1)
  calls `neighborChanged` on liquids beside, which queues a tick unless one is due at the same time (148b5cd30
  scans the pending heap). The freeze pass writes with flags 2 (no updates).
- **Border water**: after the freeze pass, in the chunk's 60 border columns (table 14a923c90) from y -64 up to the
  light heightmap, the lowest cell of each run of layer-0 water is set to a source (flags 0) and ticked in 1.

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
  drops or waterlogging, and calls `updateNeighborsAt` again.

## Oracle (2026-10-07)
Fluid cells differing near origin went 12.0k -> 1.7k. The 8-block routing removed most of our over-flow (infinite
sources BDS's water never formed, lava floods into obsidian: water-over-air cells 383 -> 71, far out 218 -> 108
Liquids errors, negative seed 42 -> 3). What remains near origin is about 1.3k cells of BDS water where ours is air:
flows BDS ran in game from chunk queues while the oracle's tiles were ticking. How long a world ticked depends on the
oracle run (a long game phase fixes near origin and breaks the small worlds), so it is not modelled.
Not modelled either: lake and monster-room writes' updates, lava's 3-in-4 recede delay (TLS random), waterlogged
plants losing water; about 100 lava cells near origin still spread where BDS's did not (a falling lava cell that
made stone below then spreads sideways in ours), unexplained.
In biome regions the liquid residue of old growth pine taiga (lush caves) and of the cave regions sits where
cave features still differ ([worldgen-bds-caves.md](worldgen-bds-caves.md)) and changed the walls a spring or a
flow needed. Bubble columns over magma grow by ticks: near a source's 8-block edge they wait for game time.
