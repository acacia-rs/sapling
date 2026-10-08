# BDS jigsaw structures (static RE + oracle, crates/sapling-worldgen/src/structure/jigsaw)

Companion to [worldgen-bds-templates.md](worldgen-bds-templates.md) (template loading and placement) and
[worldgen-bds-structures.md](worldgen-bds-structures.md) (starts). Build `bds-molang/bedrock_server.exe`
(1.26.52.3); VAs absolute; labels at the end.

## Data (`src/structure/jigsaw/pools.txt`, `pools_v2.txt`)
- Hardcoded pools (villages, ancient cities) are C++ registration code. `scripts/jigsaw_extract/jpools.py`
  runs `jemu.py` (straight-line emulation, string/vector tracking) over the ranges in its docstring;
  `jdata.py` writes pools.txt. Elements: factories 1481d8670/8940/8c00/8f00/91f0/9500/110c0 into the element
  ctor 145e18510 (projection byte, post flags byte at +0x121), plain ctor 145e132d0, feature elements
  145e137d0/1481113c0, the empty element 141f80e80 / 141f80770 (both easy to miss: the ancient city diverged
  at piece 14 without the structures pool's `empty*7`). Pools 1481d97f0 / 148111680 / 145e22c70 (weights
  expanded in registration order), alias sets 14816c9a0/c770/141f7f500. The farm crop rules are set in code
  (jdata.py `CROPS`).
- v2 pools: trail ruins are pack JSON (`__brarchive/worldgen/template_pools*`, `processors`), read by
  `jjson.py`; trial chambers are C++ (1481318c0, `jtrial.py`: element 145e1afa0 = location + the copper bulb
  list, 145e1abc0 = location + template id and no processors, pool 145e231e0(name r8, fallback r9, list of
  0x48-byte entries), registered by 141f89b50). `jtrial_data.py` adds the copper bulb rules (1480ba050:
  waxed copper bulb 0.1 / 0.33333334 / 0.5 -> waxed oxidized / weathered / exposed, lit) and the pool alias
  targets (141fa20f0; every target is one trial spawner, so the first is taken). The entrance cap pool falls
  back to itself.
- Line format: `l` block lists, `r` rule lists (`in|location|output`), `c` capped rule lists (limit), `e`
  elements (`t` hardcoded template, `j` v2 template, `f` feature), `p` pools (name, fallback, `elem*weight`),
  `a` alias sets. Templates misspelled by the C++ (`plains_acessory_1`, `taiga_lamp_post_01`) load as
  `Missing` (valid, no blocks, no jigsaws).

## Elements and processors
- Template jigsaws are in template **file** order (palette order is wrong), turned with the piece; pool
  aliases of hardcoded elements rename their jigsaws' pools.
- Hardcoded processors (`StructurePoolBlockRule`, applied in placeInWorld after the wood remap): every
  passing rule applies, later rules see the replacement; predicates draw from the settings MT (seed = the
  chunk's structure seed): random_block_match `f < p` only for 0 < p < 1, list random `f <= p`.
- v2 processors are Java's (template placement 145e1c040 -> 141ee0510, every template block processed, then
  clipped): rule processor 145944ea0 draws from a `java.util.Random` per block seeded by `Mth.getSeed` of the
  world position **with x multiplied in 64 bits**, first passing rule wins; capped 145942bd0 shuffles all
  block indices (Java shuffle) with `Random(levelSeed).forkPositional().at(template origin)` and applies its
  delegate until `limit` blocks changed (both of a list's capped processors use the same order). Jigsaws
  become their final states in place; structure blocks drop out.

## Placers (`jigsaw/assemble.rs`)
| | Legacy (villages) | Queued (ancient cities) | V2 (trail ruins, trial chambers) |
|---|---|---|---|
| RE | 141ee6920/6a70/7000/7770 | 141edde60/4b20/5600 | construct 144ce64d0, expand lambda 144d11120 |
| random | start's MT | start's MT | `java.util.Random`, large-feature seed (144ce7f40) |
| shuffle | i from 1: `u32 % (i+1)`, swap if != i | same | Java: from the end, `nextInt(i)` |
| order | depth first (recursion) | (priority, insertion) queue | same queue, jigsaw's placement priority |
| jigsaws | shuffled | shuffled, stable by selection priority | same, candidates sorted too |
| pool list | pool shuffled; fallback only if it fails or at max depth | same | pool (below max depth) + fallback, each shuffled, one list |
| rotations | [cw90, 180, ccw90, none] shuffled | same | [none, cw90, 180, ccw90] Java-shuffled |
| room test | x/z only: inside the source and clear of what went inside it, or clear of all | allowed boxes / occupied boxes; space index +1 per inside attempt | Java's: one space per piece for what goes inside, else the parent's |
- An empty element ends the pool's list. Rigid on rigid: child y = jigsaw y - candidate jigsaw y + the
  front's y step. Hardcoded terrain matching: on `surface - 1` under the jigsaw; v2: candidate jigsaw on the
  first-free height.
- V2 start: start height sampled first (uniform = `nextInt(max-min+1)+min`), rotation `nextInt(4)`, element
  `list[nextInt(len)]`, piece at (x*16, y, z*16), moved so its bottom is one below the start height (plus the
  first-free height at its middle when projected); reach box around (middle, y) clamped by the padding. The
  biome test (findGenerationPoint) reads the biome at that (middle x, y, middle z).

## Villages (`village.rs`; start 1480a83a0 -> 141f79aa0)
- Type from the biome at (x*16+8, preliminary surface, z*16+8): 2 desert, 12 snowy, 35 savanna,
  5/19/30/31 taiga, else plains. Draws: rotation, abandoned `nextFloat < 0.02`, town centre.
- Heights: the raw chunk height (`computeChunkHeightMap` 14196d010: one above the highest non-air of the
  density shape, water counts, at least -64). Start moved to height(middle) - 1.
- Piece class (141f7a9c0/ab60/abb0): skips a rigid piece whose middle two below and four bottom corners one
  below are all air; flag 1 fills under bottom-layer blocks 2-4 deep (`nextInt(3)` per filled column from
  the chunk MT) with the biome's top material (dirt for grass, nothing under leaves); flag 2 plugs the first
  hole under solid blocks within 12 of the middle (sand -> sandstone, red sand -> red sandstone, else stone).
  Terrain matching puts each block at heightmap(x, z) + template y - 1.

## Ancient cities (`ancient_city.rs`; start 1483117d0 -> 141edd690)
Rotation, centre element, then the queued placer from the `minecraft:city_anchor` jigsaw at the start
column, piece y = -27 - anchor dy - y0 - 1, within 128 of the centre, depth 7; deepslate plugs.

## Trail ruins and trial chambers (`data_driven.rs`)
- Trail ruins: pool `trail_ruins/tower`, depth 7, 80 around, start -15 projected on the surface, bury (the
  terrain stage's).
- Trial chambers (141f82140): pool `trial_chambers/chamber/end`, depth 20, 116 around, start uniform
  -40..-20, padding 10, alias set `trial_chambers`.

## Oracle (20261003-structures, `blocks_match_bds_structures`)
| kind | precision (non-air) | recall | what is left |
|---|---|---|---|
| village | 55.2% | 85.0% | the beard (terrain adaptation, not ported) and paths on the heights it moves; feature elements |
| ancient city | 84.8% | 95.5% | sculk patch feature elements; later stages over the city |
| trail ruins | 99.7% | 84.7% | the bury adaptation (terrain stage) |
| trial chambers | 99.8% | 99.6% | (other starts 97.9-99.9%; geodes overwrite one in the mansion region) |
All 840 ruin-material cells of the trail ruin match, suspicious gravel included.

## Gaps
- Feature elements (village trees and piles, ancient city sculk patches) place nothing yet.
- Village paths sit one block off in some columns (BDS's heightmap there is one higher).
- Pool alias random picks (trial chamber spawner kinds) are not drawn; blocks agree either way.

## Labels
```
141ee6920 JigsawPlacement ctor (legacy)   141ee6a70 addPieces   141ee7000 expand   141ee7770 candidates
141edde60 queued addPieces   141ee4b20 queued expand   141ee5600 queued tryPlace   141ee53d0/145e23dc0 MT shuffle
144ce7f40 v2 JigsawStructure generate   144ce64d0 JigsawAssembler::construct   144d11120 expand lambda
144cd3360 v2 shuffled jigsaws   144cd3710 weighted element list   145941160 forkPositional().at
145e1c040 v2 element place   141ee0510 v2 template processBlockInfos   145944ea0 v2 rule processor
145942bd0 v2 capped processor   1459409e0 JavaRandom::nextInt   14196c5f0 WorldGenerator::postProcessStructures
1481318c0 trial chamber pools   1480ba050 copper bulb list   141f82140 trial chambers structure
141fa20f0 trial chamber pool aliases   1480bbe90 village pools   1480fe430 ancient city pools
141f7a9c0/ab60/abb0 village piece class   141edf3a0 ancient city piece class
```
