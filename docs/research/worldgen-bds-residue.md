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

## What the deterministic rest is
- Dripstone clusters (stronghold, ancient city, igloo, badlands, cherry grove, meadow, savanna plateau): the
  largest family in every world; worked on in worldgen/liq2, not here.
- Liquids, carvers and structure-stage cells: near origin's Features stage is 767 of its 2590.
- The structures world's 6582 missing: sculk veins (ancient city 3207, pillager outpost 445, trail ruins 273),
  the jungle temple's trees and vines (1353) and the mansion's lush caves (929). Our sculk also overgrows the
  ancient city's own blocks (deepslate bricks and tiles: 988 cells ours, 2 BDS's) although 148b47d30's list has
  them; not resolved.
- Same-phase sources (jungle temple 4246, likely the mangrove chunk -499,517 and other edge cascades): two
  sources of one decoration phase are two chunks apart and both write the chunk between them. BDS decorates them
  one after the other (their 3x3 locks overlap), so the later one reads the earlier one's leaves: `VinesFeature`'s
  walker (a column from y 64, moved by four draws at every non-air cell) met one more block in BDS than in ours in
  ring chunk 78,6, which put one vine elsewhere, which let a jungle tree grow that ours refuses, which desynced
  the chunk's tree loop. sapling gives a source only the earlier phases' writes; following same-phase order
  would chain across the world, and that order comes from the load pattern, not the seed. Treat as floor.
- Fallen logs over water (trial chambers chunk 2,20): BDS places the stump and no log where ours floats two
  logs on the surface; the stump's vine roll then differs. `fits` reads the same as ours; not resolved.
- Mangrove chunk -499,517: one bee-nest mangrove draws another height; the tree before it in the rule's
  generator spends a different number of draws. Not traced further.

## Tools added
- `examples/feature_trace <seed> <cx> <cz> [substring]` (feature/trace.rs): our rule tries, tree attempts, vine
  walks and writes of a source with draw ranges. `examples/source_check`: a source's writes against BDS's cells
  per target chunk (is a ring source's work in BDS?). `examples/query_noise`, `saved_heightmaps`, `saved_chunks`.
- `examples/rule_tries` takes `RULE_ORDER=zx` for rules that draw z first and shows kelp and seagrass;
  `examples/decoration_order` takes `ANY_BLOCK=1` (it only votes on same-phase pairs now that later phases see
  earlier writes, and those rarely claim one cell).
- `tree_draws` finds a trunk's `z, x` pair anywhere in the stream: early matches are often chance. Trust
  `feature_trace`'s draw ranges for ours and a breakpoint log for BDS.
