# BDS trees (static RE + oracle, crates/sapling-worldgen/src/feature/{tree,legacy_tree.rs})

Companion to [worldgen-bds-features.md](worldgen-bds-features.md). The JSON `tree_feature` shapes are ported from
stirante/featurelab `features/tree.go` (MIT) and corrected against BDS decompiles and the oracle; the legacy tree
features, foliage, rocks, flower beds and huge mushrooms come from bdsre.

## Legacy tree features (hardcoded, registered in 1482f4580)
- `LegacyTreeFeature` (ctor 149e981a0, 0xb0 bytes; place 149e9cf40; tree loop 149e97fc0). The constructor's switch
  on a type (0 bamboo jungle .. 0x10 redwood taiga mutated) sets a float count and a list of JSON features looked up
  by name; a per-type lambda (`lambda_33`..`lambda_49`, call functions 149e9f8xx..149ea0000) picks one. Transcribed in
  `scripts/legacy_data.py` (`TREES`), pickers in `TreeChoice`. Forests use the `*_with_leaf_litter` variants (a
  version/experiment flag in 1482f4580 selects them; BDS 1.26.52 worlds have it on).
- Count: `nextInt(10) == 0` → `(int)(count * nextFloat)`, else counts ≥ 1 → `count + max(nextInt(10), 7) - 7`, else
  `nextFloat < count`. Per tree: picker, then z and x `nextInt(14) + 1` from the chunk corner, y =
  `IBlockWorldGenAPI::getHeightmap` (vtable 0xb0), the feature, then `scatter_tall_grass_around_tree_feature`
  (iterations 0) unless `isLegacyLevel` (0xb8).
- The tree loop's heightmap ignores leaves: a tree grows through another tree's lower leaves (oracle). sapling's
  `View::heightmap` (also `query.heightmap`) treats leaves as see-through.
- `LegacyForestFoliageFeature` (149e95c70, first pass): `nextInt(5) - 1` (flower forest) or `- 3` patches; per patch
  `nextInt(3)` picks lilac, rose bush, peony, then five tries x, z `nextInt(16)` and y `nextInt(heightmap + 32)`
  (absolute y: heights start at 0). `DoublePlantFeature` (148299900) draws nothing when every chunk within 7 blocks
  has its highest block (`LevelChunk` +0xf88) under `y - 4`, else 64 tries z, y, x.
- `LegacySwampFoliageFeature` (149e97c00): count (`nextInt(10)==0` → `(int)(2*nextFloat)` else `max(nextInt(10),7)-5`),
  10 seagrass tries at x, z `nextInt(16)` and y + 54, then the tree loop with `nextInt(20)==0` → huge mushroom else
  swamp oak.
- `LegacyForestRockFeature` (149e96360): `nextInt(3)` rocks at z, x `nextInt(16)`, heightmap y; the blob (14827dc10)
  sinks to grass, stone, dirt or coarse dirt, then three blobs of `nextInt(2)` per axis (radius `(a+b+c)*0.333+0.5`),
  each moved by `nextInt(2)-1`, `-nextInt(2)`, `nextInt(2)-1`.
- Leaf litter and pink petals (`minecraft:leaf_litter_feature`, `minecraft:pink_petals_feature`, vtable 14a98e630,
  place 149e82710): `nextInt(4)` growth, `nextInt(4)` facing, on air with support.
- `HugeMushroomFeature` 143b7ef50 (caps and stem 143b7f230, soil check 143b7fc40).

## Tree feature
- `TreeFeature::place` 143b7ff60: `trunk->getTreeHeight`, roots (if any), `trunk->placeTrunk(..., canopy)`.
  ITreeTrunk vtables [dtor, placeTrunk, getTreeHeight]: simple 14a98de70 (1438f6bc0), acacia 143bd34c0, fancy
  14a98d130 (1438ec690), mega 14a98d440 (1438f2ba0), mangrove 14a98d250 (1438f1c30), cherry 14a98c640 (1438ea120),
  fallen 1438ebaf0, poplar 1438f4e80; mangrove roots 1438f0d00. Canopies: cherry 1438e95c0, fancy 1438ec300, mega
  1438f2a30, mega pine 1438f2520, pine 1438f40e0, roofed 1438f5c10. TreeParams: +0x8 base blocks, +0x38 may_grow_on,
  +0x50 may_replace, +0x68 may_grow_through; BlockDescriptor match 142cf3f80.
- `TreeHelper::prepareSpawn` (1438eac70): inside the world, `may_grow_on` under the origin, then every cell of the
  trunk volume (radius 0 at the base, 1 above, 2 from one under the top to one over it) must pass the +0x68 list.
  On the oracle that list behaves as may_grow_through plus may_replace (air and leaves pass, logs fail), and as air
  and water alone when the JSON has no may_grow_through (swamp oaks fail under leaves). `Tree::grows_through`.
- `SimpleTreeTrunk::placeTrunk`: optional sink (`can_be_submerged`; with no may_grow_through it sinks through water
  only), prepareSpawn, height modifier, logs gated by may_grow_through below the original y and may_replace above, a
  failed write aborts, canopy at the top log + 1, base block below; returns the top log + 1 (where a sequence's next
  feature, jungle cocoa, starts).
- Mega trunks check may_grow_on under the origin cell only and their own volume (radius max(|w-1|,1) at the base, w
  above); dark oaks keep their logs over the origin and skip layers whose leaning column is invalid; fancy trunks
  check their lines against may_grow_through and set limb log axes per cell; cherry heights draw `nextInt(i+1)` per
  interval; mangrove trunks have no spawn check of their own (see the git log of each module for details).
- Mangrove roots run prepareSpawn over roots plus trunk (1438f0d00); a root cell may hold may_grow_through, air,
  water, leaves or replaceable plants (1438f16f0); the canopy (1438ef7d0) keeps a leaf only when its write succeeds.
- `BlockSource::setBlock` (1415f6ae0) reports whether the cell's block changed: a leaf over a leaf is not a success,
  so the mangrove canopy's leaf list (shuffle, vines, propagules) holds each changed cell once. The canopy's
  constant height and radius still draw `nextInt(1)` (inlined `min <= max`); propagules keep a box x±1, y-1±1, z±1
  free (constructor 1438ef460: +0x1e4 depth 2, +0x1e8/+0x1ec radii 1). Roots written over a water source keep the
  water (`MangroveRootsWet`).
- `TreeHelper::isValidTreePosition` (inlined) passes the block list, air, `BlockProperty::Leaves` (+0x130 & 0x20),
  Plant material the block lets be built over, and Water/Leaves material: seagrass counts as replaceable. A tree's
  `may_replace` entries are also pushed into its +0x68 list (setter 149e81320), which is why prepareSpawn behaves
  as may_grow_through plus may_replace.
- Debugging desyncs: decorate a phase-0 chunk (no neighbour writes) with `examples/trace_source` (needs the
  uncommitted trace hooks of the trees2 session: per-write lines with BDS's block), then `examples/draw_search`
  (start a try at draw k, score its writes) and `examples/mt_find` (`b:v`, `b<v`, `b>v` runs) to pin BDS's draws.

## Jungle, pale garden and flower features
- `BambooFeature::place` (14827b960): on air over grass, podzol, dirt or rooted dirt; height `nextInt(((h >> 24) %
  5) | 8) + 5`, h = `(x*0x2fc20f ^ z*0x6ebfff5)` then `h*(h*0x285b825 + 0xb)` in u32; three or more stalks get
  large, large, small leaves on top. `PodzolAreaFeature::place` (149d5fdf0): disc of radius `nextInt(4)+1` turns dirt
  and grass one below into podzol. `VinesFeature::place` (149d99a70): y 64 up to the highest block of the chunks
  within 3 columns, a vine on the first supported side (support south, north, east, west), else the column moves
  ±3 (two raw draws per axis) around the start.
- Pale garden: the pale oak is the roofed tree; `pale_moss_vegetation_feature` (149d99710, weights in 149e815b0:
  carpet 25, short grass 25, tall grass 10) and `PaleMossCarpetBlock::placeAt` (1465f64a0: per side a wall check,
  with air above a float > 0.5 makes it tall and adds the upper half); `snap_to_surface` with
  `allowed_surface_blocks` matches those blocks instead of a sturdy face (lambda 143beb0d0). Eyeblossoms
  (149e82050) are closed at generation time.
- `LegacyFlowerFeature` flower forest case (1438eec60): `(noise(x/48, z/48) + 1) / 2` clamped to 0.9999 indexes
  Java's 11-flower list, no draws. `MeadowFlowerFeature` (149e9d1d0, list in 149e9d080): palette of 1..4 noise-picked
  flowers (x*0.0005, shifted by `i*0x3936d`), one picked by the noise at x*0.1. Wildflowers are the flower-bed
  feature (149e82710) with the wildflowers block.

## Oracle (2026-10-07, after merging worldgen/residue)
Worlds `20261003-trees` (+`-run2`): 20 regions of tree biomes, test `blocks_match_bds_trees`. Logs and leaves are
100/100 (precision/recall) in birch, old growth birch, taiga, snowy taiga, snowy plains, savanna (+ plateau),
windswept forest, forest and old growth spruce taiga (podzol 99.9); dark forest 96-97 (huge mushrooms 100), cherry
95, flower forest 86-95, grove 99-100, jungle 66-84, bamboo jungle 36-66, mangrove 20-37.

2026-10-08 (worldgen/trees2): mangrove swamp 96.94 -> 98.25% identical (every tree of a phase-0 chunk matches;
later phases still cascade from neighbour seagrass/roots), tree regions 98.75 -> 98.87%, jungle and bamboo jungle
interiors in sync (left-over errors sit on the region edge rows, from ring chunks, and on one terrain hole in
bamboo jungle chunk 171,348 at (2740,100,5576)), pale garden 98.63 -> 99.40%, wildflowers 397/398.

## Left
- Mangrove: phase-1..3 chunks still drift once a neighbour's writes differ (cascade); hanging propagules BDS later
  breaks (block updates) are not modelled.
- Pale garden: the 4th tree of chunk (-469,-285) fails in BDS but not here (its trunk volume reaches chunk -468).
- Jungle edges (ring-chunk decoration), the forest case of the flower table, HugeMushroomFeature's sinking branch
  (141603120), old growth pine taiga mega spruces (767 extra leaves).
