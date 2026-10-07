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
