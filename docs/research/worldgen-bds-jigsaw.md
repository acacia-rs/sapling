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
  bindings (141fa20f0, Java's: a random group giving ranged and slow ranged the same mob of skeleton, stray,
  poison skeleton; then melee of zombie, husk, spider; then small melee of slime, cave spider, silverfish, baby
  zombie). The entrance cap pool falls back to itself.
- Line format: `l` block lists, `r` rule lists (`in|location|output`), `c` capped rule lists (limit), `e`
  elements (`t` hardcoded template, `j` v2 template, `f` feature), `p` pools (name, fallback, `elem*weight`),
  `a` alias sets, `b` a v2 alias binding (its choices, `|` apart, each `alias=pool,...`). Templates misspelled by the C++ (`plains_acessory_1`, `taiga_lamp_post_01`) load as
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

## Feature elements (`FeaturePoolElement`, vtable 14a89acf0; `src/feature/element.rs`)
- Size (0, 0, 0) (slot 0, 140906d00), so the shared box code (145e1aa90) makes an inverted box one off
  the origin by rotation: x (x, x-1) for none/ccw90, (x+1, x) for cw90/180; z (z, z-1) for none/cw90,
  (z+1, z) for 180/ccw90; y (y, y-1). It meets one chunk, or none when the origin is on that chunk's
  edge (such a feature is never placed); the foundation fill loops over no columns (no draws); rigid
  pieces still register a beard with that box. Village pine at (-1016,85,160) is the oracle's example.
- Projection 0 (rigid), post flags 3 (ctor 145e137d0 -> 145e132d0): the class check runs, then the
  feature, then plug holes.
- Legacy place (slot 8, 145e141d0, called by `PoolElementStructurePiece::postProcess` 141ef21e0): the
  feature at the piece origin with the chunk's structure generator (shared with the other pieces'
  fills), through the post-process's 3x3 block source. v2 place (slot 7, 145e144c0): its own MT seeded
  `Mth.getSeed(x, y, z) >> 16` (x times 3129871 in 64 bits).
- sapling (`structured.rs`): a chunk's features write into its neighbours as spills, seeing the
  neighbours' carved terrain; a spill lands after the target's own pieces where the replaced block is
  still there. Ordering neighbours by the decoration phases (features seeing earlier neighbours with
  their pieces) matched worse (ancient city +204 cells).
- Piles (`hay_pile_feature` and the others, one class, place 14827e1e0, block 14827ea50): Java's
  `BlockPileFeature` (`nextInt(2)` x and z radius, x, y (two layers), z; `10 f - 6 f` ellipse else 3.1%)
  on a full block top or a path (list 0x14ccd3208 = grass_path), no `nextBoolean` for paths; hay on
  axis y/x by `nextInt(2)`, packed ice 1 in 7 blue (`nextInt(7)`), pumpkins carved when a float >= 0.95.
  Poplar trees (camps): worldgen-bds-trees.md, "Poplars".
- In the v2 placer a feature element attaches to any jigsaw that faces it: the expand lambda asks the candidate
  element's vt+0x78 (1 for `FeaturePoolElement`, 1400a2880) and then calls the attach test 145e16f50 with its
  name comparison off. A tent's tree jigsaw is named and targets `minecraft:down` while the element's own
  jigsaw is `minecraft:bottom` (name and target, 145e13b90), and BDS grows that tree (oracle: the old growth
  pine taiga camp's spruce at -5315,74,5452). Java compares the names and would not.
- Its inverted box passes the room test as it is; what the space then keeps for it is the 2x2x2 box at the
  inverted box's low corner (144d13b6e: `x1 + 1 == x0` on all three axes).

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
  the chunk MT) with the biome's top material (dirt for grass, nothing under leaves); flag 2, within 12 of the
  middle, under each falling block (`BlockType` +0x163 bit 0x10 = mFalling: sand, gravel) one below the piece,
  plugs the first non-falling block if air or water (sand -> sandstone, red sand -> red sandstone, else stone).
  Terrain matching puts each block at heightmap(x, z) + template y - 1. Rigid pieces beard the terrain
  (worldgen-bds-structures.md, Terrain adaptation).

## Ancient cities (`ancient_city.rs`; start 1483117d0 -> 141edd690)
Rotation, centre element, then the queued placer from the `minecraft:city_anchor` jigsaw at the start
column, piece y = -27 - anchor dy - y0 - 1, within 128 of the centre, depth 7; deepslate plugs.

## V2 structures (`data_driven.rs`, `jigsaw/junction.rs`)
- Trail ruins: pool `trail_ruins/tower`, depth 7, 80 around, start -15 projected on the surface, bury.
- Trial chambers (141f82140): pool `trial_chambers/chamber/end`, depth 20, 116 around, start uniform
  -40..-20, padding 10, alias set `trial_chambers`, encapsulate.
- Abandoned camps (pack `abandoned_camp` set, 37/8, salt 91231127; `s` lines of pools_v2.txt from jjson.py):
  18 structures of weight 1, picked as Java's `tryGenerateStructure` does (large-feature Random,
  `nextInt(total)`, a variant whose start-piece biome fails its `has_biome_tag` filters is dropped and the pick
  repeats); start pool `abandoned_camp/tent/<biome>`, depth 3, 116 around, start 0 projected, padding 10,
  beard_thin. Tents attach a campsite, both attach tree feature elements.
- Pool aliases (`JigsawStructure::generate` 144ce7f40, Java's `PoolAliasLookup.create`): a `java.util.Random`
  of the level seed, `forkPositional().at(chunk x * 16, start height, chunk z * 16)`; each binding in order draws
  `nextInt(choices)` (resolve 145e2d4d0 for a group, 145e2d290 for a single alias). The spawner templates differ
  around the spawner (webs, bone blocks, podzol and mushrooms), so the picks show in blocks.
- V2 states (`template/java_states.rs`): v2 templates do not go through the legacy loader the state table was
  probed with, and four kinds of block come out differently (read off the oracle's trial chambers per unturned
  state and rotation, `examples/template_states`). A decorated pot's and a trapdoor's `direction` is the Java
  facing's 2D index (south 0, west 1, north 2, east 3) written as it is, the trapdoor's then turned as Bedrock
  reads it (0 east, 1 west, 2 south, 3 north), so a pot faces the other way and a trapdoor depends on its
  facing; a door's lower half is the Java facing turned (a quarter short of the legacy loader's), its upper half
  `south` turned with the piece (the legacy loader leaves it south); a trial spawner keeps
  `trial_spawner_state=1` (Java's `waiting_for_players`), which the legacy loader drops to 0.
- Pieces keep Java's ground level delta (start 1; rigid child: source delta - k, k = jigsaw y above the source
  bottom - candidate jigsaw y + front step y) and junctions (`tryPlacingChildren`), for the terrain stage.

## Oracle (20261003-structures and -biomes, 2026-10-08)
| kind | precision (non-air) | recall | what is left |
|---|---|---|---|
| village | 99.98% | 99.93% | one wall join |
| ancient city | 82.2% (own 99.5%) | 99.3% | sculk patches (thread-local in BDS, see Gaps); later stages over the city |
| trail ruins | 100% | 100% | (2026-10-09, with the walls' Java sides) |
| trial chambers | 99.91% (own 100%) | 99.94% | (2026-10-09, with aliases and v2 states; later stages over the rest) |
| abandoned camp | 93.9 / 96.4 / 99.2% | 42.5 / 96.1 / 99.7% | podzol under a camp tree; forest rocks count as wanted |
All 840 ruin-material cells of the trail ruin match, suspicious gravel included.

## Gaps
- Ancient city sculk patches are a determinism floor: the patch's walk draws depend on vein faces BDS
  picks with a thread-local generator, so the catalyst draw after it varies by run. Discriminator: the
  city feature at (-1059,-50,1235) has a catalyst in the -run2 twin and none in the main run; 20 of 69
  catalyst cells and 10286 of 36985 sculk-family cells in the region differ between the two BDS runs.
  The tally's "own" precision (cells where neither side has a decoration-family block) separates it: the
  city's own blocks are 99.5% (2026-10-08); the rest of the precision gap is sculk over city blocks, which
  BDS grows as much as sapling does, in other cells (worldgen-bds-residue.md, "Ancient city sculk").

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
145e2d090/145e2d290/145e2d4d0 alias binding resolve (direct, random, random group; vtables 14a89b340/380/3c0)
141f7a9c0/ab60/abb0 village piece class   141edf3a0 ancient city piece class
145e16f50 v2 jigsaw attach test (last argument: skip the names)   145e13b90 FeaturePoolElement jigsaw info
145e137d0 FeaturePoolElement ctor   145e141d0/145e144c0 FeaturePoolElement place (legacy/v2)
145e1aa90 StructurePoolElement::getBoundingBox   140906d00 zero getSize   14a89acf0 FeaturePoolElement vtable
14827e1e0/14827ea50 pile feature place/block   0x14ccd3208 pile path list (grass_path)
```
