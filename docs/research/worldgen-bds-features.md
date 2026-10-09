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
  (`BlockSource::getBiome` 141602f90, by id): the block's chunk in the 3x3 (rules scattering past x/z 16, such as
  sea pickles' 0..24, read the neighbour's biome), else, and outside y -64..319, the dimension's default
  (`Dimension::getDefaultBiomeId` 141882f10, vtable slot 0xc8: ocean; nether 8 and end 9 override it). So diamond
  and redstone rules that scatter below the world place (drawing their blob) while a chunk's ocean is decorated;
  treating such positions as matching nothing desynced every later blob of the rule (near origin 1.1k cells). A
  one-biome chunk places every position. Palettes fill in x, z, y storage order (1438cb860).
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
  It returns early, before any draw, when the dimension's generator reports a structure at the origin
  (`Structures::in_structure`: a mineshaft or v2 start's piece union; the oracle's four extra geodes sat in
  mineshafts and trial chambers, other kinds are untested).
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

## Feature residue (2026-10-08, worldgen/feat2)
`SAPLING_ORACLE_DUMP=<dir>` makes the oracle write every Features-stage error (and every feature cell the `-run2`
twin disagrees on) to a TSV per region; `scripts/feature_residue.py <dir> [world] [--by-region] [--cells <family>]`
splits them by family, kind (ours-extra: BDS kept the block from before features; bds-only: ours did; both) and
cause: (a) the twin disagrees (not counted as wrong), (b) thread-local generator in BDS (glow lichen and sculk
vein faces and spreads, sculk walks they steer), (c) a BDS structure block within 6 cells or an unported
structure's area holds the cell, (d) deterministic, (e) as d in a chunk on the region's edge.

| world | Features wrong | b | c | d | e | (a) |
|---|---|---|---|---|---|---|
| near origin | 2674 → 1570 | 87 | 1328 → 1227 | 1159 → 229 | 100 → 27 | 953 |
| far out | 1848 | 13 | 1765 | 0 | 70 | 102 |
| negative seed | 674 | 4 | 0 | 98 | 572 | 1331 |
| biome regions | 13581 → 12906 | 4648 | 2288 → 2106 | 1725 → 1401 | 4921 → 4751 | 3658 |
| tree regions | 22947 → 21272 | 940 | 4122 → 3979 | 4566 → 4282 | 13319 → 12071 | 2406 |
| structures | 107773 → 106095 | 8633 | 97710 → 96134 | 292 | 1138 → 1036 | 8280 |

(b) and (a) counts are before the judge stopped grading lichen/vein faces (below). In 4x4 regions 12 of 16
chunks are edge chunks, so e is not by itself an edge excess; in the 8x8 negative seed region it is (572 of 670).
Fixed: rule positions use `getBiome`'s 3x3 and ocean default (above; near origin diamond/redstone 1035 -> 0
d cells), and lush-cave clay pools match andesite, diorite, granite and grass blocks (scripts/cave_data.py:
the 1.16-format plain-string names `minecraft:stone` and `minecraft:grass` match the pre-flattening blocks;
ores' `{"name": "minecraft:stone"}` matches stone only, as the decoration-order vote showed; grove 414 -> 12
wrong). Glow lichen and sculk vein faces are not judged (`judge.rs` THREAD_LOCAL_FACES): `MultifaceFeature::place`
shuffles them with the generator at `gs:[0x58]` slot +0xe60 (143b84002), so BDS's own two runs disagree.

What is left, largest first:
- Trees in tree regions (d 2657, e 5114): bamboo jungle (chunk 171,348) and pale garden (-469,-285) each start
  from one tree BDS does not grow. Forcing our first bamboo-jungle tree (mega jungle at 2739,101,5575) to fail
  after its two height draws puts the next tree on BDS's trunk (2744,103,5570) and writes 3 -> 76 matching logs,
  so BDS's tree fails; its spawn volume (1438f2ba0: may_grow_on under the origin only, radius 1 then 2) must have
  held a non-grow-through block. Ruled out on 2026-10-08 (worldgen/coral): the phase-0 neighbours' grass rules are
  in sync (170,348's 800 tries match BDS's fern/grass picks; the "draw 4478" mismatch was a cell another rule took)
  and none of their writes (grass, vines, trees) reaches the volume; BDS's fern, vine and oak there come later in
  171,348 itself (the oak is a fancy oak starting at draw 328 of the same tree loop). Neither tree is in a
  structure. Both refused trees are 2x2 trunks over a one-deep drop (air under one or two trunk cells), but
  requiring ground under all four cells made dark forest far worse (BDS grows most such trees). Rule order by
  FNV-1a instead of FNV-1 (the rule store's other hash) made every world worse, so FNV-1 stands.
  Pale garden: the acacia-style trunk (143bd34c0) runs prepareSpawn (1438eac70) only; BDS's tall grass in its
  volume comes from no earlier-phase source either.
- Dripstone (biomes d 565, e 2537; trees d 430, e 3105): about half within 8 cells of a lichen or sculk
  difference (column scans stop at lichen); mostly edge chunks.
- Edge chunks generally: ring-last decoration (ring chunks after every region chunk) made every region worse
  (near origin 3379 -> 5642 wrong, negative seed 677 -> 2205), so the parity model holds for the ring too; the
  negative seed's kelp/seagrass cluster (chunks 0,3 and -1,3) is a draw desync of kelp and deep warm-ocean
  seagrass whose first divergent try is a seagrass try over a neighbour's kelp column.
- Coral: ported (worldgen-bds-caves.md, "Coral"). Geodes in structures: fixed (above); far out 99.9636 →
  99.9787%, near origin 99.9960 → 99.9971%, geode shells 100/100 in every world.
- Other d families after that (2026-10-08): mangrove swamp trees (358) and old growth pine taiga trees (651);
  both open, see worldgen-bds-trees.md ("Refused 2x2 trees", "Mangrove chunk -500,515").
- Explained on 2026-10-08 (worldgen/feat3), none a feature bug:
  - Mushroom fields coal (87): our terrain is one block lower at the single column (-693,72,5674) (mycelium
    over dirt one cell down), so blob #14 of `coal_ore_lower` in chunk -44,354 counts one stone cell fewer and
    every later blob of the rule shifts; starting #13 one to three draws later takes the chunk from 47 to 3 wrong
    (`draw_search`). A shape/surface difference, not the mineshaft.
  - Tuff in badlands and frozen peaks: every cell is tuff on one side and sculk on the other (sculk floor).
  - Sulfur in windswept gravelly hills (137 cinnabar turned sulfur by spike clusters): 97 lie within 10 blocks of
    a glow lichen difference and the `-run2` twin itself has our block at some of them; cinnabar is replaceable
    (1720 such writes in the region's chunks agree with BDS). The clusters' column scans stop at lichen.
  - Dripstone: the same mechanism; e.g. badlands chunk 596,-349's cluster #63 desyncs beside a lichen BDS's two
    runs place differently, and the next cluster (#21) is then wholly off.
  `feature_residue.py` now files these as b (lichen or sculk on either side) and f (dripstone or sulfur within 10
  blocks of a lichen or sculk difference): biome regions d 2107 → 1667 + f 1074 of the old d/e, tree regions
  f 1641, structures f 2577.
- Template data markers (structure/template/markers.rs) insert a new key in front of its bucket's first element,
  as MSVC's `_Find_last` scans a bucket back to its low end; the structures oracle did not change.
- Near origin d (229): bubble columns (111, grown by ticks), lichen spreads (69), lava/water hardening (47).
- 2026-10-09 (worldgen/liq2): dripstone split per feature instead of by distance (worldgen-bds-caves.md,
  "Dripstone residue: floor and deterministic"); the lava/water hardening cells went with the liquid ticks'
  neighbour-update rule (worldgen-bds-liquids.md). The table above is from before the frozen heightmaps.
