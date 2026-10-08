# BDS feature rules (static RE + oracle, crates/sapling-worldgen/src/feature)

Companion to [worldgen-bds.md](worldgen-bds.md). Feature rules run in `decorationPostProcessChunk` after lava lakes.

## Data
- Rules and features are the vanilla pack JSON: `behavior_packs/vanilla_<ver>/__brarchive/{feature_rules,features}
  .brarchive` (256-byte entries: length byte + name, u32 offset/size at +248) plus `definitions/` (old 1.13 files).
  The 1.18+ ores (`*_lower`/`*_upper`, tuff, copper) exist only in the packs; the old unsplit rules there have
  iterations 0. `scripts/features_to_rust.py` transcribes them into `src/feature/data/` (the BDS install is not in
  the repo, and rule and feature data is static); biome filters are evaluated there against each biome's tags, as
  BDS does when it attaches rules to biomes at load. Rules reaching an unported feature are kept with a placeholder
  that places nothing: their `math.random` iterations draw from the generator all rules of the chunk share (glow
  lichen's `Math.Random(104, 157)` runs before every after-surface rule). `scripts/features_data.py` turns feature documents
  and the hardcoded features into `FEATURES`, reading the block table of `src/block.rs` for block names.
- Rule store (loader 144c03360): `unordered_map<pass, unordered_map<HashedString id, rule>>`, filled from the pack
  stack newest first (brarchive entry order), then `definitions/` by name; the first rule of an identifier wins.
  Each biome's decoration list is the rules whose filter passes, in the inner MSVC map's iteration order (bucket =
  FNV-1 64 of the full identifier & mask, 8 buckets, x8 growth). The script simulates the map; alphabetical order
  loses 0.5% of feature cells (overlapping stone blobs).

## Decoration of one chunk
- `decorateWorldGenPostProcess` 14821f400: per pass (`first_pass` .. `final_pass`), `BiomeDecorationSystem::
  decorate` 14580bb90 over the chunk's biome list (1438c8450: sub-chunk biome palettes bottom up, first seen
  first), `decorateBiome` 14580b170 per biome over its list, skipping other passes.
- Per rule: seed = boost `hash_combine(chunkSeed, (u32)FNV1_64(identifier))` with the large-feature chunk seed;
  MT1 draws the scatter (chance gate, iterations, one position per iteration, index counting down, axes in
  `coordinate_eval_order`, origin = chunk min corner, y 0); MT2, seeded alike, is handed to every placement.
- With more than one biome in the chunk, a position places only if its block's biome is the one being decorated
  (`BlockSource::getBiome` 141602f90, by id). Outside y -64..319 that returns the dimension's default biome, which
  sapling treats as matching none (oracle: diamond and redstone rules scatter below the world; 73-81% to 99%
  precision). A one-biome chunk places every position. Palettes fill in x, z, y storage order (1438cb860).
- Scatter (`ScatterParams::initScatter` 14478ea70, `getPosConst`/`getPosMolang` 14478fbe0/14478fe20,
  `sampleDistribution` 14478f9f0): extents `roundf`; uniform `min + nextInt(max-min)`, no draw if max <= min;
  triangle `min + nextInt(R/2+1) + nextInt(R-R/2+1)`; percent gate draws only for 0 < pct < 100.
- Molang in rules: `variable.origin*` is the chunk corner, `variable.world*` each axis as drawn, `math.random` draws
  `nextFloat` from the chunk's own generator (seeded with the chunk seed, shared by all rules of the chunk; 14478e670
  `ScatterParams::initMolangParams` captures it), `variable.is_legacy` is 0. `query.heightmap(x, z)` (lambda
  1411a2740) is `LevelChunk` +0xb80 (the light heightmap) plus the minimum y: one above the highest block that is not
  air or a see-through plant, water included. `query.noise(x, z)` (1411a24c0) is one simplex octave of an MT seeded
  2345 (any world), unscaled.
- `ore_feature` (`OreFeature::place` 143b8dcb0): featurelab's port (Java legacy vein in f32 centred on origin+8,
  the Mth sine table, sphere pruning, cell-centre test, x/z/y loop); `may_replace` by block type; per matching
  (cell, rule) a discard roll when 0 < chance < 1, drawn before the 6-face test for `minecraft:air` (143b7fdc0).
- Disks (hardcoded, at `query.above_top_solid` = one above the highest motion-blocking block, fluids passed,
  140a08bb0): `SandFeature` 149d60440 (sand r 7, gravel r 6) and `ClayFeature` 148287a70 (r 4) need water at the
  position (clay: or clay), draw `nextInt(r-2)+2`, replace dirt, coarse dirt and grass (clay: mud instead of
  grass) within 2 (clay 1) blocks vertically, x then z then y. `GrassDiscFeature` 148299570 (mangrove swamp, r 5)
  tests nothing at the position and always succeeds: dirt, coarse dirt, grass and mud within 2 become grass under
  air, dirt elsewhere (so mud under water turns to dirt). Its mud id 0x14cce09b8 has no `lea` in the initialiser;
  it sits between magenta and orange concrete powder in an alphabetical run (once misread as mycelium).

## Wrapper features (IFeature vtable: dtor, place(ctx), isValidPlacement, upgradeFormat)
A placement context is {world-gen API, position, generator, Molang scope, ...}; children get their parent's
generator, so a rule's whole feature tree draws from its second MT in call order.
- `scatter_feature` (143b9b5d0): optional `project_input_to_floor` (down while the block below is air), then a
  copy of the scope with the position as `variable.origin*` and `math.random` on the placing generator; chance,
  iterations and positions as for rules (old format at the top level, 1.21.60 under `distribution`), each axis
  relative to the position. Returns the last child's success.
- `aggregate_feature` / `sequence_feature` (`AggregateFeature<0/1>` 144c1b820/144c1ba00): children at the same
  position / at the previous success; `first_failure` stops only while nothing has succeeded yet (it tests the
  accumulated result), `first_success` returns at once.
- `weighted_random_feature` (143bcfb10): weights summed as `(int)((float)sum + w)`, `nextInt(sum)` (none if 0),
  first child whose running `(float)r - w` goes negative.
- `single_block_feature` (143badfc0): `may_not_attach_to` fails first (no draw); then one `nextInt(total weight)`
  even for a single block; placement rules (world-gen API slot 0x78 `mayPlace`), survival rules (0x88
  `canSurvive`), `may_replace`, `setBlock`. Survival as ported (plants.rs): grass, ferns, flowers, double plants on
  dirt-like blocks, a double plant's lower half also needs air above, dry plants and dead bushes on sand and
  terracotta too.
- `search_feature` 143ba64d0, `vegetation_patch_feature` 143bc8310, `growing_plant_feature` 144c15200: see Trees and
  cave.rs. `snap_to_surface_feature`, `multiface_feature`, `surface_relative_threshold_feature` and
  `partially_exposed_blob_feature`: [worldgen-bds-caves.md](worldgen-bds-caves.md).

## Vegetation (hardcoded features, registered in 1482f4580; vtable slot 4 is place(BlockSource, pos, Random))
Tries move by `(r & m) - (r & m)` per axis, two raw MT outputs each; most draw z, y, x (±7, ±3, ±7) after the try.
- Kelp (149e915a0): down until the block below is solid-blocking (or kelp), then per try: climb to water or air, skip
  without drawing on air, water below or a column under 3 water blocks; length `nextInt(depth - 3) + 3` (3 when depth
  is 3), ages from `26 - length - 1` up to 24; move z then x ±7 only after a full column.
- Seagrass (149d61510): down to a solid-blocking block, then per try: climb to water or air; if water and solid
  below, `nextFloat < tall` places a double (water above needed), else a single; biomes under 0.15 need one more water
  block. Always moves z then x ±7 after the try. Variants: seagrass_feature (tall 0.6), shortest 0.3, shorter 0.4,
  taller 0.8.
- 1482f4580 sizes kelp/seagrass (1 or 10 tries) and flowers, mushrooms, sweet berries (64 or 1) from one flag
  that static reading did not settle; the oracle fits 1 try for all (10 kelp tries triples kelp, 64 flowers floods).
- Dead bush (1482947e0): its descent reads before decrementing, so the base ends one below the ground; 4 tries.
- Sugar cane (149d60080): 20 tries x then z ±7 at the rule height, needing water beside the block below; height
  `nextInt(nextInt(3) + 1) + 2`. Cactus (14827f3f0): 10 tries, height `nextInt(nextInt(3) + 1) + 1`, the cactus-flower
  flag is off. Melon (149e9d840): 64 tries on grass blocks. Lily pad (14829d6c0): no draws, on water under nothing.
- `BlockPileFeature` (148287420): pumpkins (96 tries, `nextInt(4)` cardinal direction) and sweet berries
  (`nextInt(8)` growth) on grass blocks; offsets x, z, y.
- Mushrooms (149d57200): on mycelium, nylium, podzol or logs anywhere, else only with 13 points of a radius-2
  diamond in shade (149d9a560: under the light heightmap, or a not fully translucent block within 10 above).
- Flowers (`LegacyFlowerFeature` 149e95670, table 1438ef3b0): nothing unless the position is air; one flower per
  call: swamp blue orchid; overworld `nextInt(3) == 0` dandelion else poppy; plains noise(x/200, z/200) < -0.8 gives
  a tulip `nextInt(4)` (red, orange, white, pink), else `nextInt(4) > 0` picks poppy, azure bluet, oxeye daisy,
  cornflower by `nextInt(4)`, else dandelion. Flower forest and meadow: see worldgen-bds-trees.md; the forest table is not ported.
- Waterlogged plants keep water in the second layer; seagrass and kelp are not water material.

## Geodes, icebergs, freeze pass
- `geode_feature` (`GeodeFeature::place` 149e845f0, row lambda 149e867f0): Java 1.17's geode in f32 on the placing
  MT. Draws: points `min + nextInt(max - min)` (the maximum is never reached, as for every range here), then the
  NormalNoise (-4, [1]) in legacy mode from that MT (`PerlinNoise::from_mt`: octave 0's ImprovedNoise first, 262-draw
  skips, `nextDouble` = one raw draw x 2^-32), the crack float and the crack chance (0.95 and base size 2 hardcoded),
  per point x, y, z offsets and the point offset. Layers 1/sqrt(2.2|3.2|4.2 + d), filling 0x3f4457d1; inverse
  square roots are the float 0x5f3759df with one Newton step. The scan walks x from the origin up then down, per x
  y up then down, z inner; cracks are set at once, layers buffered and set after the scan, buds tried up, down,
  north, south, west, east (14a7445a0). Anchors in air, liquids, ice, packed/blue ice or bedrock count as invalid.
  It returns early when the position is inside a structure (query at 149e845f0's start): not modelled.
- `legacy:ice_feature` (149e96790): `nextInt(8) == 0` places `IcebergFeature` (149e8f460, cut-out 149e90f80) at
  `x + 12 + nextInt(8)`, sea level, `z + 12 + nextInt(8)`; then `nextInt(20)` `BlueIceFeature` (14827ec60) tries at
  `nextInt(16) + 8`, `nextInt(32) + 30`, `nextInt(16) + 8`. The iceberg is Java 1.17's with every shape draw up front
  (11 - nextInt(5), 3 + nextInt(3), nextInt(6), nextInt(15), ellipse when a float > 0.8, ...), no `getEllipseC`
  narrowing, a 1% blue-ice body; blue ice grows only next to 1..4 blue ice blocks.
- Freeze pass (end of `decorationPostProcessChunk`, x outer, z inner): the rain top (140a09950, first block from the
  top that is not air or a plant) and `Biome::getTemperature` (14119c8c0): frozen oceans (`partially_frozen`) take
  0.2 where `sum 3 octaves (seed 3456, x0.05) + 1 octave (seed 2345, x0.2) < 0.3` and the 2345 noise at x0.09 is
  below 0.8; above y 80 temperature falls by `(y + c/8 - 80) * 0.05 / 40`, c the chunk's per-column byte
  `(i8)(noise(seed 1234, x/8, z/8) * 64)` (1411a3e60). 14119c8c0 picks sea + 17 and x8 against sea + 1 and x4 on
  version flags; the oracle fits +17/x8 (the other branch snows 1.1k cells too low in windswept gravelly hills).
  Water under the top freezes at <= 0.15 (`shouldFreeze` 14119e860); the top snows (`shouldSnow` 14119e740) over a
  motion-blocking block other than ice and powder snow (packed ice holds snow).

## Oracle (2026-10-07, 20261003 near origin unless named)
Precision/recall near origin: stone blobs 99.93/99.98, coal 99.79/99.90, seagrass 99.95/99.93, kelp 99.89/99.88,
diamond 97.0/97.2, redstone 98.6/98.5, geode shells 98.0/100 and buds 99.0/100 (BDS skips geodes inside
structures), grass and ferns 99.8/99.9, flowers, sunflowers, sugar cane 100. Biome regions: geode shells and buds
100/100 everywhere; dead bushes and dry grass 98.5-100, lily pads 85-97, mushrooms 85 (taiga) and 100, cacti 60/75.
The diamond and redstone shifts sit beside mineshafts: their blobs draw a discard float per exposed cell, so
structure air BDS has and sapling lacks shifts the rest of the rule's draws. Cave features (speleothems, glow
lichen, magma, monster rooms, sulfur pools, ice spikes) and what is left: [worldgen-bds-caves.md](worldgen-bds-caves.md).

## Trees
See [worldgen-bds-trees.md](worldgen-bds-trees.md). `search_feature` (143ba64d0) runs its feature over the box in the
axis's loop order until `required_successes`; `single_block_feature`'s `may_attach_to` (feature/attach.rs) checks
top, bottom and diagonals hard and counts the four sides, auto-rotating toward the last match. A `sequence_feature`
stops at its first failed step. A rule's first `scatter_feature` sees no biome (its Molang environment is copied
before the world is bound), so its `query.has_biome_tag` chance is 0 and draws nothing. `query.above_top_solid` and
the tree loop's heightmap skip leaves.

## Decoration order
BDS decorates a chunk once its 3x3 neighbourhood is loaded and lets its features write into those 9 chunks; writes
further out are dropped. The scheduler (`ChunkSource::_spawnChunkGenerationTasks` 1409ddfc0) takes the pending
{chunk, stage} set, sorts it by `ChunkBuildOrderPolicy::getChunkRebuildPriority` (Dimension +0x3b0, vt slot 1; MSVC
`std::sort` 140a0cbd0) and launches from the highest; the decoration task (140a16cd0) runs
`decorationPostProcessChunk`, which first takes `PostprocessingManager::tryLock` (1438df8f0) on the 3x3 and, failing,
returns the chunk to the queue (1409e3920); success unblocks neighbours (1409e09d0). Ticking areas register a box
influence (142fe8f60, even-aligned bounds). What the oracle shows, with no exception in 3.3k neighbour pairs over
all four worlds (`examples/decoration_order.rs`: votes from cells two chunks' stone-only ore blobs both claim):
neighbours decorate in parity phases, x even z even first, then x odd z even, both odd, x even z odd. Chunks of a
phase are never neighbours; their mutual order (same phase, two apart) is not measured. The oracle's ring of
chunks around each region is decorated too (leaving its features out makes every region worse).

sapling models that order and it is what a server-side generator should use (it depends on the seed only): each
source chunk is decorated over its 3x3 as the earlier phases left it (the chunk's terrain before features plus the
replayed writes of every lower-phase source around it, cached per chunk and phase), and logs each write with the
block it replaced; a column replays the 9 sources around it by phase, then x, then z, applying a write only where
the replaced block is still there. The freeze pass runs at the end of each source's decoration, over its own
columns. Near origin this took wrong cells from 103.8k to 8.1k (seagrass 74.8 -> 99.9%, coal 90.7 -> 99.8%).
Cost: per column one decoration (cached) plus up to three phase snapshots per neighbour chunk; the oracle suite went
from 85 s to 160-210 s on testbox (geodes and icebergs included). perf then showed terrain recomputation, not the
replays: `SourceCache` cleared every entry when full, the aquifer maps hashed with SipHash, biome picks ran in
single-biome lattice cells; with those fixed (`examples/bench_columns.rs`, 16x16 near origin) `chunk_blocks` costs
31.5 ms per column (was 47.4; terrain alone 5.1) and the suite, liquids included, runs in about 75 s.
