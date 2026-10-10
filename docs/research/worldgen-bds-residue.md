# BDS feature residue and its floor (2026-10-09, worldgen/feat4 and feat5)

Companion to [worldgen-bds-features.md](worldgen-bds-features.md). What the oracle still counts wrong after the
feat4 fixes, how much of it BDS's own nondeterminism explains, and the tools that found each cause. The newest
round is first; the sections after it are feat4's and the sculk round's, with their numbers as they were.

## misc: fossils, camp trees, poplars (2026-10-10, worldgen/misc)
- Fossils ported (worldgen-bds-features.md, "Fossils"): desert pyramid region 108 -> 41.
- The old growth pine taiga camp (43 cells): the v2 placer attaches a feature element whatever the jigsaw's
  target name, so the tent's `down` jigsaw grows a tree (worldgen-bds-jigsaw.md, "Feature elements"). The
  "other positions" this file listed were that tree, a decoration spruce, and a piece on a chunk edge neither
  side places. Region 668 -> 620; nothing of the camp is left.
- Poplars and shelf mushrooms ported and checked through a pack override (worldgen-bds-trees.md, "Poplars").
- The template palette no longer names states no registry has (worldgen-bds-templates.md).
- Heightmaps stay carved for decorated neighbours too (worldgen-bds-features.md, "Heightmaps"): the swamp's
  firefly bushes and a lily pad under a neighbour's oak (swamp 79 -> 72, mangrove swamp, pale garden, mansion).
- Mushrooms do not stand on leaves (the camp's last cell and eight more in five regions).

All2 (02df143) -> misc, wrong / missing / deterministic per world; nothing else moved:

| world | wrong | missing | deterministic |
|---|---|---|---|
| near origin | 2019 | 313 | 2024 |
| far out | 259 | 29 | 260 |
| negative seed | 98 | 12 | 78 |
| biome regions | 7405 -> 7401 | 1512 -> 1468 | 1909 -> 1861 |
| tree regions | 3955 | 1073 | 3026 |
| structures | 22882 -> 22881 | 6692 -> 6626 | 11049 -> 10982 |

(The feat5 table below is from before the drip and struct merges; these are the integrated tree's numbers.
`tools/oracle.sh <name>` and `tools/oracle-totals.sh <before> <after>` reproduce them.)

## feat5: terrain to the bit, icebergs, and what the judge hid (2026-10-09)
Fixed, with the region each showed in (wrong + missing, 06ed458 -> feat5, the judge as it was):
- Terrain shape (worldgen-bds.md, "Terrain port findings"): blended noise offsets, the 3D spaghetti's thickness
  span, the pillars' cube. Every one-block column error is gone (mushroom fields 132 -> 44, near origin 2423 ->
  2332, plains, dark forest, snowy taiga, windswept savanna 8 -> 1, and the ore blobs those columns shifted).
- Frozen ocean icebergs (worldgen-bds-surface.md): 64-bit position hash, temperature at sea level, truncated
  bounds, and ice counting as sea in the overworld builder (frozen ocean 300 -> 84, deep frozen ocean 186 ->
  26, plus 2.3k cells the judge had left to "iceberg features" and its neighbours).
- Bee nests (`single_block_feature` with placement rules, worldgen-bds-features.md): mangrove swamp 630 -> 251;
  the edge tree at (-7979,64,8276) was the nest of the rule's first tree, placed over that tree's own root.
- Snow over small plants (freeze pass): snowy plains 209 -> 128, snowy taiga 561 -> 511, the igloo region
  1561 -> 1435.
- Mushrooms on logs: the far-out oak (362 -> 287); firefly bush clusters (ported; swamp 190 -> 185).

| world | wrong | missing | deterministic |
|---|---|---|---|
| near origin | 2081 -> 2019 | 342 -> 313 | 2115 -> 2024 |
| far out | 329 -> 259 | 33 -> 28 | 334 -> 259 |
| negative seed | 98 | 15 | 81 |
| biome regions | 10895 -> 10207 | 1636 -> 1442 | 4997 -> 4115 |
| tree regions | 9362 -> 9188 | 801 -> 783 | 6770 -> 6579 |
| structures | 31365 -> 31206 | 6482 -> 6463 | 16800 -> 16622 |

Terrain-stage errors over all worlds (`scripts/terrain_residue.py` on the dump's `.terrain` files): Shape 62 ->
12, Carvers 28 -> 12, Surface 45 -> 41. None of the rest is terrain: Shape is an abandoned camp's podzol (2) and
stone where lava met water (10); Surface is cinnabar where a spike cluster BDS placed made sulfur (36) and magma
under a bubble column (5); Carvers is a village path in the stronghold region (9), a lake's obsidian neighbour
and one diamond ore.

### What the judge hid
The oracle's "left to later stages" came from when features were not ported. Its buckets fell from 12510 to
9495 cells with the fixes above, and reading them by reason found whole bugs:
- "iceberg features" (1739), "freeze pass on a column top" (394), disks and blobs under icebergs (187): the
  frozen ocean builder. The iceberg feature itself was right all along.
- "surface under a solid-shape difference" (6442 left): the judge asked about a reshaped column before it asked
  whether BDS's block is one only features place, so dripstone blocks and sculk under such columns never
  counted. "ore features (or a missed vein)" (1083 left): lush-cave clay over stone (about 500), and the BDS
  side of every ore blob that shifted.
`judge.rs` now counts a feature-only block as a feature error under any column, and BDS's ice, snow, ores,
disks, podzol and dirt over our terrain as missing; what stays "later" is multiface faces, terrain under a
reshaped column, cells BDS emptied beside a feature block, waterlogged cells and geodes over carved cells.
Judged that way (06ed458 -> feat5; later rounds compare against the right-hand numbers, "later" is 1900 cells):

| world | wrong | missing | deterministic |
|---|---|---|---|
| near origin | 2081 -> 2019 | 370 -> 313 | 2143 -> 2024 |
| far out | 329 -> 259 | 34 -> 29 | 335 -> 260 |
| negative seed | 98 | 15 | 81 |
| biome regions | 13138 -> 11269 | 2848 -> 1659 | 7729 -> 4671 |
| tree regions | 10614 -> 10441 | 1378 -> 1342 | 8117 -> 7908 |
| structures | 35544 -> 35322 | 6821 -> 6782 | 19737 -> 19476 |

The stricter judge also showed snow layers on a pillager outpost's fences and torches (the freeze pass now
wants a full cube under a structure block: 3209 -> 3141 there) and a fossil we do not place (60 bone blocks
under the desert pyramid region: `minecraft:fossil_feature` is not ported).

### Traced, not ours to fix here
- Coal ore in meadow (101) and badlands (145): the first coal blob that differs (`examples/try_check`, sources
  371,-508 and 594,-349) holds a cell that is a dripstone block in BDS, placed by an earlier-phase neighbour's
  cluster we do not place there. The blob counts one stone cell fewer, and every later blob of the rule
  shifts (ours-only coal counted wrong, BDS-only coal counted missing). It follows the dripstone residue.
  Cherry grove (204) was not traced that far: its first differing blobs each have one cell BDS left stone.
- Sulfur in windswept gravelly hills (306) and savanna (91): spike clusters, as in feat4 (scans stop at
  lichen).
- Old growth pine taiga, chunk -333,340 (43): BDS's abandoned camp grows a spruce at (-5315,74,5452) and one at
  (-5308,·,5457); ours has feature elements at (-5307,74,5456) and (-5303,74,5454) instead. Jigsaw assembly.
- One firefly bush cluster (5051..5053,63,4944) stands under oak leaves in BDS: the neighbour chunk's
  heightmap it read had no leaves there, ours has. Resolved on 2026-10-10 (worldgen/misc): a decorated
  neighbour's heightmap is not recomputed (worldgen-bds-features.md, "Heightmaps").
- Bamboo jungle's and the pale garden's edge chunks: a few dozen cells of podzol, grass and vines after the
  lush caves and dripstone are set aside; not traced.

## Fixed on feat4 (cause, where it showed)
- Ore veins (`OreFeature::place` 143b8dcb0): before the sphere test BDS keeps only cells within the widest
  sphere's radius of the vein's axis (a 2D test per column, then 3D per cell; `Axis` in feature/ore.rs). Near the
  origin the gate never bites; at x, z 1.6M the sphere centres round to 1/8 block off the line and it drops edge
  cells (far out: 997 wrong -> 336).
- Molang origin (`decorateBiome` 14580b170): `initMolangParams` (14478e670) sets `variable.origin*` to the chunk
  corner once per biome and pass, not per rule, and writes through a pointer the features share, so a
  `scatter_feature` (143b9b5d0, its other caller) leaves the origin at its own position. A later rule's
  `iterations` then reads that position: in a two-biome ocean chunk the kelp rule's second run counted
  `query.noise` at the seagrass rule's position (negative seed 1357 -> 113). Found by logging the arguments of
  the noise call (breakpoint 1411a25f1) and matching the odd ones to our trace's rule positions.
- Glow lichen placed in water keeps the water (`GlowLichenWet*`, `Block::into_water`); dry lichen was washed away
  by our liquid pass. Worldgen sculk veins replace the water (the oracle has them dry). Mangrove leaves and roots
  keep it too, also over seagrass (`View::set_block_quiet`).
- Vines (`VinesFeature` 149d99a70): bamboo is no support (lush.rs `sturdy`; bamboo jungle 2181 -> 358).
- Melons (149e9d840) go through `BlockType::mayPlace`: they replace ferns, grass and vines.
- Mushrooms' shade (149d9a560): under the heightmap, or any block that is not fully translucent
  (`BlockType` +0x15c < 1) within ten above. With the heightmap frozen, the second test is what sees the leaves
  of a tree the chunk has just grown.
- Kelp and seagrass descend past a structure's fences and stairs (not solid-blocking; shipwreck 531 -> 59).
- Heightmap contents: see worldgen-bds-features.md, "Heightmaps" (leaves count; settled).

## Floor: what BDS itself does not repeat
`tests/oracle_support/floor.rs` prints one `floor:` line per region and `scripts/oracle_floor.py <log> [<earlier
log>] [--regions]` sums them per world. A stable wrong or missing cell is attributed to the thread-local
generator (`gs:[0x58]` +0xe60: multiface faces and spreads) when
- either side holds glow lichen, a sculk vein or a sculk block (`lichen/sculk`; `(twin)`: of those, the ones
  within 15 blocks of a lichen or sculk cell on which BDS's two runs disagree, 15 being a sculk cursor's reach), or
- it is a dripstone, sulfur or lush-cave block (features whose column scans and `snap_to_surface` searches stop
  at lichen) within 10 blocks of such a twin disagreement (`scans (twin)`).
`deterministic` is the rest; `(upper)` keeps the lichen and sculk cells the twin does not confirm. Final oracle
(3c2167f), wrong + missing before -> after, then the split:

| world | wrong | missing | wrong + missing | lichen/sculk (twin) | scans (twin) | deterministic (upper) |
|---|---|---|---|---|---|---|
| near origin | 2220 -> 2239 | 579 -> 351 | 2799 -> 2590 | 310 (241) | 0 | 2280 (2349) |
| far out | 997 -> 336 | 39 -> 33 | 1036 -> 369 | 28 (20) | 0 | 341 (349) |
| negative seed | 677 -> 98 | 680 -> 15 | 1357 -> 113 | 15 (15) | 17 | 81 (81) |
| biome regions | 11012 -> 11033 | 1819 -> 1657 | 12831 -> 12690 | 6179 (5912) | 1377 | 5134 (5401) |
| tree regions | 11202 -> 9445 | 911 -> 806 | 12113 -> 10251 | 1235 (934) | 2165 | 6851 (7152) |
| structures | 32354 -> 32025 | 7150 -> 6582 | 39504 -> 38607 | 16161 (15382) | 4925 | 17521 (18300) |

(Wet lichen moved near-origin cells from missing to wrong: a lichen we now place on another face than BDS's is
not graded, one in another cell is.) The split is a lower bound on the floor: a scan stopped by a lichen shifts
every later draw of its rule, so cells further than 10 blocks follow, and they are left in `deterministic`.

Whole sculk patches come and go between BDS's runs (jagged peaks: 325 sculk cells in the twin only, all in the
region's east column), so sculk is floor even where a run's cells look stable. The saved ring chunks are all
`FinalizedState` 2 in both runs (`examples/saved_chunks`): the ring is decorated either way.

## After worldgen/drip (2026-10-09)
The dripstone family was one bug, not floor (worldgen-bds-caves.md, "Dripstone residue": `snap_to_surface`'s
surface names, then waterlogged spikes and supports). Oracle at the branch's end, all2 (3e2439e) -> drip:

| world | identical % | wrong | missing | wrong + missing | lichen/sculk | scans (twin) | deterministic |
|---|---|---|---|---|---|---|---|
| near origin | 99.9975 -> 99.9975 | 2081 -> 2081 | 342 -> 342 | 2423 -> 2423 | 308 | 0 | 2115 |
| far out | 99.9935 -> 99.9935 | 335 -> 335 | 33 -> 33 | 368 -> 368 | 28 | 0 | 340 |
| negative seed | 99.9981 -> 99.9982 | 98 -> 98 | 15 -> 12 | 113 -> 110 | 15 | 17 | 78 |
| biome regions | 99.9455 -> 99.9583 | 10895 -> 7499 | 1636 -> 1603 | 12531 -> 9102 | 6054 | 373 | 2675 |
| tree regions | 99.9613 -> 99.9828 | 9373 -> 3983 | 801 -> 781 | 10174 -> 4764 | 1233 | 676 | 2855 |
| structures | 99.9444 -> 99.9603 | 31830 -> 22114 | 6563 -> 6560 | 38393 -> 28674 | 16187 | 1575 | 10912 |

The split's `scans (twin)` column fell with the errors (biome regions 1378 -> 373, tree regions 2160 -> 676,
structures 4882 -> 1575): cells it had filed as floor because a twin disagreement lay within 10 blocks were the
snap bug's. Treat both floor columns as "not disproved", and check a family's own precision before trusting them.
A phase-0 chunk run alone under the debugger is the test that separates a port difference from BDS's run
(worldgen-bds-debugger.md): the dripstone and lush roots tried after the fixes showed two port differences, a
rail taken for a floor (fixed) and stronghold terrain one block off (structure stage).

## What the deterministic rest is
- Dripstone: see above; what is left of it (biome regions 169 stable cells, tree regions 519, structures 1819)
  follows the lichen's thread-local cell and same-phase order.
- Liquids, carvers and structure-stage cells: near origin's Features stage is 767 of its 2590.
- The structures world's 6582 missing: sculk veins (ancient city 3207, pillager outpost 445, trail ruins 273),
  the jungle temple's trees and vines (1353) and the mansion's lush caves (929). Sculk over the ancient
  city's own blocks is as common in BDS as in ours ("Ancient city sculk" below; the "988 ours, 2 BDS's" once
  written here was a misreading).
- Same-phase sources (jungle temple 4246, likely the mangrove chunk -499,517 and other edge cascades): two
  sources of one decoration phase are two chunks apart and both write the chunk between them. BDS decorates them
  one after the other (their 3x3 locks overlap), so the later one reads the earlier one's leaves: `VinesFeature`'s
  walker (a column from y 64, moved by four draws at every non-air cell) met one more block in BDS than in ours in
  ring chunk 78,6, which put one vine elsewhere, which let a jungle tree grow that ours refuses, which desynced
  the chunk's tree loop. sapling gives a source only the earlier phases' writes; following same-phase order
  would chain across the world, and that order comes from the load pattern, not the seed. Treat as floor.
- Fallen logs over water (trial chambers chunk 2,20): fixed, see "Structures-world residue".
- Mangrove chunk -499,517: one bee-nest mangrove draws another height; the tree before it in the rule's
  generator spends a different number of draws. Not traced further.

## Structures-world residue (2026-10-09, worldgen/sculk)
The items above marked "not resolved", and what a live server said about each. Wrong + missing, 3e2439e -> c3bd232:

| world | wrong | missing | lichen/sculk (twin) | scans (twin) | deterministic |
|---|---|---|---|---|---|
| structures | 31830 -> 31365 | 6563 -> 6482 | 16165 (15386) | 4882 | 17346 -> 16800 |
| tree regions | 9373 -> 9362 | 801 | 1233 (932) | 2160 | 6781 -> 6770 |
| far out | 335 -> 329 | 33 | 28 (20) | 0 | 340 -> 334 |

Near origin (2081 + 342), the negative seed (98 + 15) and the biome regions (10895 + 1636) did not move.
By region: trial chambers 574 -> 268, mansion 3872 -> 3725, ocean ruins 236 -> 189, pillager outpost 2772 ->
2747, trail ruins 1113 -> 1104, jungle temple 4242 -> 4231. Structure tallies (non-air precision / recall): trial
chambers 99.75 / 99.78 -> 99.91 / 99.94 (own 100), trail ruins 99.73 / 99.40 -> 100 / 100, pillager outpost
99.68 / 99.93 -> 100 / 100, mansion 99.907 / 99.907 -> 99.974 / 99.974, the mansion region's trial chamber own
99.09 -> 100.

### Ancient city sculk: no rule difference
"988 city cells ours turns to sculk, 2 BDS does" was one line of the tally read alone. Over every cell our
structure stage leaves a city block in (deepslate bricks and tiles, cracked, polished, cobbled;
`examples/city_sculk`), sculk in ours / BDS / BDS's twin run:

| ours, BDS, twin | all three | ours + BDS | ours + twin | ours only | BDS + twin | BDS only | twin only | none |
|---|---|---|---|---|---|---|---|---|
| cells | 4174 | 161 | 272 | 956 | 947 | 259 | 292 | 11794 |

Ours converts 5563 city cells, BDS 5541, the twin 5685: the same amount, 4335 of them the same cells. A
breakpoint on the conversion (`SculkVeinBehaviour` 148b4265b, old block in rax) in chunk -70,78 logged 462
conversions, 441 of them city blocks (deepslate tiles 161, bricks 146, cracked tiles 68, cracked bricks 66),
every one from the deep dark's `sculk_patch_feature` rule (frames 143ba1747, 14580b883), none from the
city's own patch elements, as in ours. Nothing runs before the pieces, no other tag, nothing protected.
What differs is which cells: ours against either BDS run 2434 and 2356 cells, the two BDS runs against each
other 984. Ours is further from BDS than BDS is from itself, so not all of it is the thread-local floor; the
one measured bias is veins: 10195 in ours against 8850 and 8782 (inside the chunks' interior: beside sculk
2538 against 2072, on terrain 4148 against 3210, on city blocks 890 against 1133). The spread
(`getSpreadFromFaceTowardDirection` 1465e8f20) was re-read and agrees with ours, including the fall-through to
the next spread type that Java lacks. Not found.

### Same-phase sources and the jungle temple: load order, measured
Lock log of the jungle temple region (worldgen-bds-debugger.md): decoration ran in parity phases for 1030 of
1030 neighbour pairs; of same-phase pairs two apart, the lower coordinate went first in 441 and the higher in
507. Chunk 80,6 was decorated before 78,6. The slowed server missed the script's deadline, the tile was
retried, and BDS decorated 78,6 a second time in the same process: 8 of the 50 `VinesFeature` calls of the two
executions differ (the first walked higher in each: its ceiling, `LevelChunk` +0xf88 of the chunks
within 3 columns, was higher), and both differ from ours from call 13 on, where the oracle world's execution
differs from ours in one call. One BDS, one chunk, three outcomes: floor, as assumed.

### Mineshaft drift: structure post-processing has no parity
worldgen-bds-structures.md, "Structure post-process order": half of all neighbour pairs run against the
phases and 17% flip between two runs, so reads past the chunk cannot be matched. The frozen heightmaps did not
change this: the read is of a neighbour's planks, not of a height.

### Fixed
- Fallen logs (`fallen_trunk` 1438ebaf0): the descent and the fit test ask the world-gen API's slot 0x90
  (1411a09d0 -> `BlockType` vt+0x298, "can be built over"), true for liquids. Over a pond the log sinks to the
  bed and, as in trial chambers chunk 2,20, usually does not fit there: a stump and no log.
- Trial chambers: spawner pool aliases are drawn and v2 templates convert four kinds of state their own way
  (worldgen-bds-jigsaw.md, "Pool aliases", "V2 states").
- Walls and outposts: worldgen-bds-templates.md ("Walls", "Pillager outposts").

### Left in the structures world
- The cobblestone and lava beside the trial chambers (x 32..44, y -46..-40, z 336..340; 41 cells) is lava
  meeting water, flowing in from ring chunk 1,20, which the oracle did not save. Not a monster room: no
  spawner.
- Ancient city: about 30 single cells where a degradation rule fell otherwise (slabs, stairs, cracked tiles),
  and about 30 redstone cells BDS has ticked (powered repeaters, wire signal, extended pistons).
- Mansion: 25 wall cells on chunk corners (order of structure post-processing).
- Mineshafts everywhere (stronghold, igloo, ocean ruins, city): draw drift from reads past the chunk.

## Tools added
- `examples/city_sculk` (city cells by who turned them to sculk, vein counts), `examples/template_states` (v2
  template states against BDS per rotation), `examples/pieced_column` (a column before connections are
  joined), `bds_blocks` with `BDS_LIST=1`; `feature_trace` shows monster room tries.
- `examples/feature_trace <seed> <cx> <cz> [substring]` (feature/trace.rs): our rule tries, tree attempts, vine
  walks and writes of a source with draw ranges. `examples/source_check`: a source's writes against BDS's cells
  per target chunk (is a ring source's work in BDS?). `examples/query_noise`, `saved_heightmaps`, `saved_chunks`.
- `examples/rule_tries` takes `RULE_ORDER=zx` for rules that draw z first and shows kelp and seagrass;
  `examples/decoration_order` takes `ANY_BLOCK=1` (it only votes on same-phase pairs now that later phases see
  earlier writes, and those rarely claim one cell).
- feat5: `examples/try_check <seed> <world> <cx> <cz> <substring>` (per rule try and per tree of a tree loop,
  our writes against what BDS saved in those cells: the first try that differs is where to look);
  `SAPLING_ORACLE_DUMP` also writes `<world>.<region>.terrain` (every stable error of a stage before the
  features) for `scripts/terrain_residue.py` (clusters with stages and pairs); `examples/column_density` (our
  block densities around a cell), `density_dump_diff`, `octave_offsets` and `climate_dump_diff` (BDS's dumped
  buffers against ours, worldgen-bds-debugger.md).
- `tree_draws` finds a trunk's `z, x` pair anywhere in the stream: early matches are often chance. Trust
  `feature_trace`'s draw ranges for ours and a breakpoint log for BDS.
