# BDS cave features, the sulfur caves' subsurface builder and monster rooms (crates/sapling-worldgen)

Companion to [worldgen-bds.md](worldgen-bds.md) and [worldgen-bds-features.md](worldgen-bds-features.md). Static RE of
`bedrock_server.exe` 6ab54bee plus the oracle worlds; labels at the end.

## Sulfur caves: the subsurface noise gradient (surface/gradient.rs)
- loadChunk's second surface pass (148211300, after `buildChunkSurface`, flag vt+0xbe0): per column, from y -64 up to
  the preliminary surface minus 8 (clamped), the column is split into runs of one biome (a run's end is its last
  cell; a lone biome cell at the top is dropped). Runs of biomes with a `minecraft:subsurface_builder` (only sulfur
  caves) go to it, from one below the run's top down to its start.
- `minecraft:noise_gradient` (`NoiseGradientSurfaceBuilder` 1487e14c0, noise made in 1487e0c20 as a NormalNoise from
  the world's positional factory by name, here `minecraft:sulfur_cave_gradient` -5 [1, 0, 1]): per non-air,
  non-liquid block not in `non_replaceable_blocks`, the noise at the world block clamped to [-1, 1]; the first band
  with `min < v <= max` sets the block (cinnabar, sulfur, cinnabar).
- Not ported: the same builder as a top-of-column surface builder (sulfur caves never top a column in the oracle).

## Rules with unported features
A rule's iterations may be `Math.Random(a, b)`, drawn from the generator every rule of the chunk shares. Rules are
therefore never dropped: an unported feature becomes `Feature::Unported` and its rule still draws its scatter.
Before this, glow lichen's `Math.Random(104, 157)` (after_underground) desynced every later Math.Random rule.

## Speleothems (feature/speleothem/)
One class each for pointed dripstone and sulfur spikes, parameterised by base block, spike block and replaceable
list (VanillaBlockTypeIds lists 14ccd2770: dirt, coarse dirt, stone, tuff, deepslate; 14ccd27a0: sulfur, cinnabar).
- `SpeleothemFeature::place` (149d62d10): tip direction from base blocks below/above (both: `nextBoolean`, true =
  pointing up), base patch at the root (6 faces: `nextFloat >= 0.3`, then `!nextBoolean` twice with `nextInt(6)`
  steps), `nextFloat < 0.2` for two tall; `growPointedDripstone` (14829e980) writes base..tip into air or water only.
- `SpeleothemClusterFeature` (149d61dc0): height `[3, 5]` (sulfur `[1, 4]`), wetness `ClampedNormal(0.1, 0.3, 0.1,
  1.0)` (sulfur all 0; 1436d7cd0 always draws a gaussian), density `nextFloat*0.4+0.3`, radii `nextInt(6)+2` x then
  z; per column (x outer) `ColumnUtils::scan` 12 (1438eb800: inside = air or water, edge = base or lava), wet
  `nextFloat`, pools by `canPlacePool` (14829e3d0: stone/deepslate/tuff/water around), `nextDouble < chance` per
  spike, layers of `nextInt(2)+2` base blocks, Java 1.18's height law and meeting draw, `nextBoolean` for merging.
- `LargeSpeleothemFeature` (149e94510): radius `[3, 18]` capped by 0.33 of the room, bluntness 0.3..0.9 /
  0.4..1.0, scale 0.4..2.0, wind 0..0.3 with Mth's sine table, `moveBackUntilBaseOrLava` (149e94ab0), placeBlocks
  (149e94de0) replaces open cells and spikes, stalagmites below the light heightmap.
- MT `nextGaussian` is the polar method in f32 with the second value cached in the generator (`+0x9d4`).

## Wrapper features (feature/cave.rs, multiface.rs, blob.rs)
- `snap_to_surface` (143bb7720, search 143bb7ac0): surfaces ceiling, floor, random_horizontal (sides 2, 5, 3, 4
  shuffled forward with raw draws `% (i+1)`), random_vertical (`nextBoolean`). From an open origin the scan walks
  allowed open cells (air if `allow_air_placement`, water if `allow_underwater_placement`) to a sturdy face (or an
  `allowed_surface_blocks` block); from a solid origin it walks solid cells (`allow_non_air_placement`) to an open
  one. Every scanned cell must keep the origin's biome (`scanColumnWithinPosBiome`). Placement is beside the
  surface, or in it with `embed_in_surface` (offset `solid ^ embed`).
- `multiface_feature` (143b83e50, attach 143b84880): origin air or water; the first face (of the pushed list:
  floor, ceiling, walls N E S W) whose neighbour `can_place_on` takes decides; success only when the cell changes,
  then one `nextFloat` (spread gate). Else each face's neighbour, retried `search_range` times with the same result.
  BDS shuffles faces with a thread-local generator and spreads with it too: the two oracle runs disagree on faces;
  sapling keeps the pushed order and leaves spreads out (lichen recall about 50% near origin, precision 77%).
- `surface_relative_threshold` (143bc69b0): place only below the quart column's preliminary surface minus the depth.
- `partially_exposed_blob` (featurelab's): magma 1 around the floor, one `nextFloat` per cell; magma under water
  starts a bubble column up through water sources (BDS grows it by ticks: near a source's edge it stops short).

## Decoration and other hardcoded features
- Monster rooms (feature/monster_room.rs, before the feature rules on the post-lake generator): Java 1.12's dungeon,
  flags 3, chest tries z then x, chest facing per 149e9dae0 (south, west, north, east), loot and mob draws.
  Missing: the lava lake's border draws (`nextInt(2)` per upper border cell) before the rooms.
- `sulfur_pool` = `LakeFeature(water, sulfur, [sulfur_spike])` (149e91f50/149e923b0) over the view: legacy blobs,
  fits (no liquid above y 4, solid or the liquid below, no avoided block), fill, then the solid border becomes the
  border block (y >= 4 by `nextInt(2) != 0`). Potent sulfur via search_feature.
- Ice patch (149e8de60) and ice spike (149e8e4e0): Java 1.12's; packed ice also takes grass blocks.

## Oracle (2026-10-08, BDS 1.26.52.3, cells left to later stages or wrong, by family)

| family | near origin | far out | negative seed | biomes (20) | trees (20) |
|---|---|---|---|---|---|
| sulfur, cinnabar | 0 | 0 | 0 | 132688 → 324 | 260052 → 7302 |
| sculk (not ported) | 0 | 0 | 0 | 26102 → 26145 | 7677 → 7685 |
| speleothems | 0 | 0 | 0 | 8957 → 2378 | 12883 → 4457 |
| ice spikes | 0 | 0 | 0 | 8952 → 2447 | 0 |
| monster rooms | 2476 → 167 | 498 → 312 | 471 → 0 | 772 → 192 | 2151 → 2 |
| magma, bubble columns | 2467 → 211 | 177 → 0 | 76 → 50 | 373 → 37 | 124 → 2 |
| glow lichen | 1000 → 586 | 89 → 33 | 69 → 26 | 469 → 267 | 420 → 150 |
| coral, pickles (not ported) | 0 | 0 | 0 | 1631 → 1641 | 0 |

Identical cells: near origin 99.9788 → 99.9857%, far out 99.7574 → 99.7667%, negative seed 99.9475 → 99.9784%,
biome regions 99.1900 → 99.6639%, tree regions 98.7500 → 99.6023% (windswept gravelly hills 91.43 → 99.97%,
savanna 83.30 → 99.47%, ice spikes 99.48 → 99.95%).
Family precision / recall (stable cells): monster rooms 98.8/93.1 near origin, 100/100 in most regions; magma
95.6/97.9 and bubble columns 91.4/96.7 near origin; dripstone 66-82/67-83 (badlands, windswept hills); sulfur spikes
73.6/90.1; packed ice (ice spikes) 99.7/94.2; glow lichen 77.0/50.0 near origin.

## Left
- Sculk (`sculk_patch_feature` 143ba12e0): Java 1.19's `SculkSpreader(true, 50, 1, 5, 10)` (ctor 148b47ba0,
  `updateCursors` 148b48870 with ChargeCursor.update inlined; behaviours default 14a94bec0, sculk 14a94bf00, vein
  14a94bf40), neighbour offsets shuffled forward on the placing generator. About 30% of deep-dark cells differ
  between the two BDS runs. Needs sculk vein's 64 face states, sensors/shriekers/catalyst and the multiface spreader.
- Coral (warm ocean): `CoralCrustFeature` 148288070, `CoralFeature` 14828af70, `CoralHangFeature` 148294070,
  `SeaAnemoneFeature` 149d60d10, `SeaPickleFeature` 149d61020; Bedrock's own, not Java's.
- Sulfur springs (`random_sulfur_spring_with_tuff_feature`: structure templates) and the spring trail.
- Fossils (`fossil_feature`, desert/swamp 1 in 64 chunks) and the desert well: structure templates.
- Glow lichen spreads (thread-local in BDS; the single-candidate cases are deterministic) and lichen faces.
- Dripstone residue: world-state differences (lichen spreads, sculk) shift whole clusters' column draws.
- The ~100 lava cells that spread sideways after a fall made stone below: BDS's `_tick` sets stone with flags 3 and
  returns like sapling (148b5d060); not resolved.
- Bubble columns by ticks (edge routing) and the lava lake's border pass.

## Labels (scratch bdsre `names.user.json`)
1487e14c0/1487e0c20 `NoiseGradientSurfaceBuilder::{buildSurfaceAt,init}`, 149d62d10 `SpeleothemFeature::place`,
14829ecd0/14829e980/14829e7d0/14829e6d0/14829dfb0 `DripstoneUtils::{getTipDirection,growPointedDripstone,
getPointedState,getDripstoneHeight,isCircleMostlyEmbedded}`, 149d61dc0/14829e3d0 `SpeleothemClusterFeature::{place,
canPlacePool}`, 1436d7cd0 `ClampedNormalFloat::sample`, 1438eb800/1438eb960 `ColumnUtils::{scan,scanDirection}`,
149e94510/149e94ab0/149e94de0 `LargeSpeleothem{Feature::place,::moveBackUntilBaseOrLava,::placeBlocks}`,
143bb7720/143bb7ac0 `SnapToSurfaceFeature::{place,_findSnapPosInDirection}`, 143b83e50/143b84880
`MultifaceFeature::{place,placeBlockIfPossible}`, 143bc69b0 `SurfaceRelativeThresholdFeature::place`,
149e8de60/149e8e4e0 `{IcePatch,IceSpike}Feature::place`, 143ba12e0 `SculkPatchFeature::place`, 148b47ba0/148b48870
`SculkSpreader::{SculkSpreader,updateCursors}`, coral place functions above, 144a184d0
`OverworldGenerator::getGenerationHeight`.
