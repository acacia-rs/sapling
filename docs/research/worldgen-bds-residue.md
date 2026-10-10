# BDS feature residue and its floor (2026-10-09, worldgen/feat4)

Companion to [worldgen-bds-features.md](worldgen-bds-features.md). What the oracle still counts wrong after the
feat4 fixes, how much of it BDS's own nondeterminism explains, and the tools that found each cause.

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

## After worldgen/struct4 (2026-10-10)
Support classes from the block registry, the vein feature's full `can_place_on` and the sculk growth box
(worldgen-bds-caves.md, "Support" and "Sculk"). all2 (038b471) -> struct4:

| world | identical % | wrong | missing | wrong + missing | lichen/sculk | scans (twin) | deterministic |
|---|---|---|---|---|---|---|---|
| near origin | 99.9975 -> 99.9975 | 2081 -> 2080 | 342 -> 342 | 2423 -> 2422 | 307 | 0 | 2115 -> 2115 |
| far out | 99.9935 -> 99.9935 | 329 -> 329 | 33 -> 34 | 362 -> 363 | 29 | 0 | 334 -> 334 |
| negative seed | 99.9982 -> 99.9982 | 98 -> 98 | 12 -> 12 | 110 -> 110 | 15 | 17 | 78 -> 78 |
| biome regions | 99.9583 -> 99.9652 | 7499 -> 6082 | 1603 -> 1278 | 9102 -> 7360 | 4346 | 373 | 2675 -> 2641 |
| tree regions | 99.9828 -> 99.9852 | 3972 -> 3448 | 781 -> 639 | 4753 -> 4087 | 567 | 676 | 2855 -> 2844 |
| structures | 99.9610 -> 99.9695 | 21649 -> 16806 | 6479 -> 5089 | 28128 -> 21895 | 10122 | 1575 | 10366 -> 10198 |

By region: ancient city 13911 -> 9000, frozen peaks 1826 -> 396, pillager outpost 2709 -> 1903, trail ruins
1101 -> 642, savanna plateau 750 -> 431, cherry grove 498 -> 269, stony peaks 2292 -> 2092, meadow 478 -> 372;
no region rose by more than a cell or two. The whole gain sits in the `lichen/sculk` column: that column was
called floor, and a third of it was one support predicate and one truncated block list. It is "cells holding a
thread-local block", not a measure of what the port cannot reach.

What was looked at and is not a port difference:
- Stronghold corridor with BDS air one above ours (-1439,-3,-2010): a mineshaft corridor's ceiling row
  (`generateMaybeBox` 0.8, one float per cell). `examples/mineshaft_draws` on chunk -90,-126 has the first
  three pieces in sync and later ones best at -8 and +8 draws, the eight cobweb floats of a section whose
  support test reads past the chunk: the mineshaft drift of "Structure post-process order", not terrain.
- Stronghold liquids (659 cells): a waterfall from an aquifer pond at (-1473,47,-1893) that BDS ran 27 levels
  down in game ticks; decoration's instaticks stop where ours does (worldgen-bds-liquids.md, game time).
- Outpost and trail ruins water against air (about 160 cells): water flowing over a vein of ours that BDS's
  thread-local feature put elsewhere.
- Vein spreads at a hashed direction: worse (worldgen-bds-caves.md, "Sculk").

Left, deterministic and unported: the desert fossil (60 bone blocks at -6088,-51,-3515; `fossil_feature`
places structure templates), 117 snow layers over short grass beside the snowy-plains igloo (snow keeps the
plant in the second layer), grindstone, skull, shelf and end rod support by state.

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
- `tree_draws` finds a trunk's `z, x` pair anywhere in the stream: early matches are often chance. Trust
  `feature_trace`'s draw ranges for ours and a breakpoint log for BDS.
