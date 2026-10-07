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
- An instatick needs every chunk within a block of the cell in the 3x3; otherwise it is requeued with delay 0 (time
  `MAX`, never due in game). Leftovers go back to the chunk's queue; in game they run at the next ticks.
- While instaticking, `neighborChanged` reaches only blocks with BlockLegacy +0x163 bit 3, which `LiquidBlock`'s
  constructor (148b5c4c0) sets: liquids do hear their neighbours. `spread_to` replaces whatever it enters without
  drops or waterlogging, and calls `updateNeighborsAt` again.

## Oracle (2026-10-07)
Near origin the fluid cells differing went 12.0k -> 1.4k (Liquids stage 88.7% of 12.8k cells). The rest is order:
our water forms infinite sources BDS's does not (and then floods lava into obsidian), or stops a cell short; no
missing tick source stood out. Not modelled: lake and monster-room writes' updates, lava's 3-in-4 recede delay (TLS
random), in-game ticking of leftovers, waterlogged plants losing water.
