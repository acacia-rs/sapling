# BDS feature rules (static RE + oracle, crates/sapling-worldgen/src/feature)

Companion to [worldgen-bds.md](worldgen-bds.md). Feature rules run in `decorationPostProcessChunk` after lava lakes.

## Data
- Rules and features are the vanilla pack JSON: `behavior_packs/vanilla_<ver>/__brarchive/{feature_rules,features}
  .brarchive` (256-byte entries: length byte + name, u32 offset/size at +248) plus `definitions/` (old 1.13 files).
  The 1.18+ ores (`*_lower`/`*_upper`, tuff, copper) exist only in the packs; the old unsplit rules there have
  iterations 0. `scripts/features_to_rust.py` transcribes them into `src/feature/data.rs` (the BDS install is not in
  the repo, and rule and feature data is static); biome filters are evaluated there against each biome's tags, as
  BDS does when it attaches rules to biomes at load. Rules reaching any unported feature are left out (each rule
  has its own generators, so leaving one out shifts no other). `scripts/features_data.py` turns feature documents
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
  grass) within 2 (clay 1) blocks vertically, x then z then y.

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
- `search_feature` 143ba64d0, `snap_to_surface_feature` 143bb7720 (its search 143bb7ac0), `vegetation_patch_feature`
  143bc8310, `growing_plant_feature` 144c15200, `multiface` 143b83e50, `surface_relative_threshold` 143bc69b0: located,
  not ported.

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
  cornflower by `nextInt(4)`, else dandelion. Forest and flower forest tables (noise x/48) not ported.
- Waterlogged plants keep water in the second layer; seagrass and kelp are not water material.

## Oracle (2026-10-07, 20261003 near origin unless named)
Precision/recall: grass and ferns 99.3/99.3, kelp 93.8/95.9, seagrass 74.8/75.1 (59-84 elsewhere), flowers
87.4/92.4, sunflowers 82.8/85.7, bushes 100/98.3, sugar cane 100/67; biome regions: dead bushes and dry grass
98.5-100, lily pads 85-97, mushrooms 85 (taiga) and 100 (mushroom fields), sweet berries 100, cacti 60/75.
Seagrass misses are decoration order: a source chunk whose scatter overlaps a neighbour's that BDS decorated
first meets its seagrass, skips the float draw and desyncs. Neither x- nor z-major neighbour orders fit (each
fixes some chunks and breaks others). Lush caves (moss, clay pools, dripleaves, vines; 1.4k grass and 8.7k clay
cells in the negative-seed region) need snap_to_surface, vegetation_patch, growing_plant and the hardcoded
moss_vegetation and dripleaf features.

## What trees need
Surface-pass rules place hardcoded `legacy:*_tree_feature`s (`LegacyTreeFeature` by type, registered in 1482f4580),
JSON `tree_feature` (grove pines), aggregate/weighted trees (mangrove, cherry, pale oak, roofed forest), beehives
(search_feature over a weighted pick of aggregates of single blocks with `may_attach_to`), forest rocks and the
`legacy:*_foliage` first-pass features. Trees change what later rules see: leaves raise the light heightmap and
shade mushrooms, trunks turn grass to dirt, giant spruces lay podzol; pine taiga grass recall (48%) and mangrove
grass (26%) wait on them.

## Cross-chunk scheme
BDS decorates a chunk once its neighbours are generated and lets its features write into the 3x3 chunks around it;
writes further out are dropped (oracle: allowing two chunks is no better). Which neighbour decorates first depends
on load order, so cells two chunks' features both claim, and draws that depend on blocks a neighbour's features
changed, are not reproducible (the `-run2` worlds mask some of it).

sapling decorates each source chunk alone, over its 3x3 neighbours' terrain before features (cached loaded
columns), and logs each write with the block it replaced. A column replays the logs of the 9 source chunks around
it, x then z, applying a write only where the replaced block is still there, so the first source to claim a cell
keeps it. A column depends only on the seed, not on generation order. Seeding a source's view with its
neighbours' writes (several orders tried) changes nothing measurable for ores and makes seagrass worse overall.
Cost: per column one decoration (cached for the neighbours) plus the loaded columns of a 5x5 area, amortised to one
per column when generating an area; chunk biomes (a byte per block) are cached too and shared by the surface, the
decoration and the caller (whole column 27.4 -> 21.7 ms on testbox, 20261003 chunks 0..7, cycles -21%; the
vegetation rules cost nothing measurable there).
