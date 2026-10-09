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
- A spike set into a water source keeps the water in the second layer (`LevelChunk::setBlock` 1409f59e0;
  `Block::into_water`, the `PointedDripstone...Wet` blocks). It stays solid and is not "water" to a column scan,
  but `canPlacePool` counts it: the column after a pool with a stalagmite in it may pool too (oracle, cherry grove
  5538,4,-7219).

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
  sapling keeps the pushed order. A spread (1465e9360, after `nextFloat < chance_of_spreading`) tries the four
  directions in thread-local order: sapling places it only when exactly one direction can take it.
- Multiface spreading (feature/spreader.rs): `getSpreadFromFaceTowardDirection` 1465e8f20 tries same cell (face
  `to`), same plane (cell + `to`, face `from`), around the edge (cell + `from` + `to`, face opposite `to`);
  `getStateForPlacement` 148b68b30 adds a face only over a sturdy neighbour. Configs: lichen 14a8b32f0 (into air,
  own block, water source), vein 14a94cdb0 (not onto sculk or a catalyst; anything replaceable; non-veins are always
  valid sources). `spreadAll` walks faces up, down, N, S, W, E and the same order toward.

## Sculk (feature/sculk/)
- `SculkPatchFeature::place` 143ba12e0: origin sculk, vein, or air/water source beside a solid block; rounds of
  `cursor_count` cursors (charge split by 1000, at most 32), `spread_attempts` x `updateCursors` (148b48870, veins
  spread in the first `spread_rounds`), then catalyst (`nextFloat <= 0.5`, solid below, flags 2) and extra
  shriekers (`min + nextInt(max - min + 1)`; without `extra_growth_chance` sapling assumes 0..0, one draw).
- Spreader `(worldgen, 50, 1, 5, 10)`. Cursor: delay 0, decay 1, facings unset (-1); empty (-2) on sculk. Per update:
  behaviour by the block (default 14a94bec0, sculk 14a94bf00, vein 14a94bf40), spread veins, use charge, discharge
  or move to the last unobstructed sculk/vein of the 18 non-corner offsets (table 0x14a94bf78, forward shuffle
  `raw % (i+1)`), first vein with a face on `sculk_replaceable_world_gen` (148b47d30) wins; beyond dist^2 224 the
  charge is lost. Never merged in worldgen.
- Sculk charge 148b41810: `nextInt(5)`; growth when not at the origin, air/water above and < 3 sensors/shriekers in
  the cursor's +-4/+-1/+-4 box (`nextInt(50) < charge`, shrieker 1 in 11), else `nextInt(10)` decay by distance.
  Vein charge 148b42070: six faces shuffled inside-out (1403747f0), the first face on a replaceable block becomes
  sculk, veins around it lose faces on sculk (148b42aa0), veins spread from it; else `nextInt(5)` halves.
- The patch's walk is chaotic: BDS's sculk_vein_feature faces are thread-local, so whole patches diverge even on
  cells both BDS runs agree on.
- `surface_relative_threshold` (143bc69b0): place only below the quart column's preliminary surface minus the depth.
- `partially_exposed_blob` (featurelab's): magma 1 around the floor, one `nextFloat` per cell; magma under water
  starts a bubble column up through water sources (BDS grows it by ticks: near a source's edge it stops short).

## Decoration and other hardcoded features
- Monster rooms (feature/monster_room.rs, before the feature rules on the post-lake generator): Java 1.12's dungeon,
  flags 3, chest tries z then x, chest facing per 149e9dae0 (south, west, north, east), loot and mob draws. They
  continue the lava lake's generator after its border pass (stone, `nextInt(2)` per border cell at y >= 4).
- Sea pickles (149d61020, aquatic.rs): down to a solid block, four tries 0..5 off it (z then x), `nextInt(4)` count.
- `sulfur_pool` = `LakeFeature(water, sulfur, [sulfur_spike])` (149e91f50/149e923b0) over the view: legacy blobs,
  fits (no liquid above y 4, solid or the liquid below, no avoided block), fill, then the solid border becomes the
  border block (y >= 4 by `nextInt(2) != 0`). Potent sulfur via search_feature.
- Ice patch (149e8de60) and ice spike (149e8e4e0): Java 1.12's; packed ice also takes grass blocks.

## Oracle (2026-10-08, BDS 1.26.52.3, cells left to later stages or wrong, by family)

| family | near origin | far out | negative seed | biomes (20) | trees (20) |
|---|---|---|---|---|---|
| sulfur, cinnabar | 0 | 0 | 0 | 132688 → 324 | 260052 → 7302 |
| sculk | 0 | 0 | 0 | 26102 → 5070 | 7677 → 917 |
| speleothems | 0 | 0 | 0 | 8957 → 2378 | 12883 → 4457 |
| ice spikes | 0 | 0 | 0 | 8952 → 2447 | 0 |
| monster rooms | 2476 → 167 | 498 → 312 | 471 → 0 | 772 → 192 | 2151 → 2 |
| magma, bubble columns | 2467 → 211 | 177 → 0 | 76 → 50 | 373 → 37 | 124 → 2 |
| glow lichen | 1000 → 586 | 89 → 33 | 69 → 26 | 469 → 227 | 420 → 140 |
| coral, pickles (pickles only; coral ported later, see Coral) | 0 | 0 | 0 | 1631 → 1642 | 0 |

Identical cells: near origin 99.9788 → 99.9857%, far out 99.7574 → 99.7667%, negative seed 99.9475 → 99.9784%,
biome regions 99.1900 → 99.7265%, tree regions 98.7500 → 99.6229% (windswept gravelly hills 91.43 → 99.97%,
savanna 83.30 → 99.47%, ice spikes 99.48 → 99.95%, stony peaks 99.36 → 99.85% with sculk).
Family precision / recall (stable cells): monster rooms 98.8/93.1 near origin, 100/100 in most regions; magma
95.6/97.9 and bubble columns 91.4/96.7 near origin; dripstone 66-82/67-83 (badlands, windswept hills); sulfur spikes
73.6/90.1; packed ice (ice spikes) 99.7/94.2; glow lichen 77.0/50.0 near origin.

## Coral (feature/coral/, 2026-10-08)
BDS's own features (no Java counterpart); each rule has its own generator, so hang and anemone depend on crust and
coral only through the blocks they leave. Block lists: 0x14ccda7e8 the 10 coral blocks (tube, brain, bubble, fire,
horn, live then dead), 0x14ccd9410 the 5 live ones, 0x14ccd9440 plants, 0x14ccd9458 fans, 0x14ccd3250 wall fans.
"Water" is the main block's material; `BlockSource::isUnderWater` (1416027c0) for a solid coral block means the
liquid-or-main block above is water and none of the 5 other neighbours is air (`fits`), for a fan or plant that the
cell's liquid layer is water.
- `CoralFeature::place` (14828af70): up to the first water below sea level - 4, solid floor that is not live coral,
  colour `nextInt(5)`, `_placeCoral`, then each `topDec` cell (water, no water under it, a support) gets a fan
  (`nextInt(5)`, `coral_fan_direction` `nextInt(2)`) and each `sideDec` (cell, face) draws `nextInt(3)` of a face table
  (0x14a925e68) and, on water beside a coral or stone block, a wall fan of colour `nextInt(2) + 2*nextInt(3)` (clamped
  to 4), `coral_direction` by face (0x14a7198f8).
- `_placeCoral` (14828b9e0) draws `nextInt(3)` and falls through like a switch without breaks: shape 0 builds all
  three shapes, shape 1 the last two. Every write goes through `_canOverwrite` (148290a30: not a coral block of
  another colour; failing ends that shape). Shape 0 stacks star layers (`_starFormation` 1482914f0) but writes every
  layer's centre at the base, then a stem with up to three `_buildPlantArm` (148291d20) arms on distinct faces;
  shape 1 is a cornerless square on solid ground, the left-out (-x,-z) corner, a cross and a short stem; shape 2 a 3x3
  floor and a short stem (6 in 7) or a tall stem with three runs along a face. BDS's own oddities kept: the (-j,+j)
  diagonal tried twice, side decorations recorded at the arm's base.
- `CoralCrustFeature::place` (148288070): templates `coralcrust/crust1..5` (by size `nextInt(5)`, turned `nextInt(4)`;
  probe and placement offsets 0x14a925dc0..e64) in a cluster of `nextInt(5) + 4` tries on solid ground, or one
  `outcropping1..6` (`_getCropOffsetFromRot` 148289370) 1 in 4 or when no try lands. Templates come from
  scripts/coral_data.py (dead coral lands as dead bubble coral, oracle census). `_placeCoralBase` (1482895b0) grows
  coral from the template's data markers in MSVC `unordered_map<BlockPos>` order (with rehash; a key goes in front of
  its bucket's first node), then decorates like `CoralFeature`.
- `CoralHangFeature::place` (148294070): down to a solid coral or stone block above sea level - 13, a side in water
  (E S W N, each failed pick swapped out of a shrinking `nextInt`), a tube or brain wall fan; BDS tests the liquid beside
  the rule's position, not beside the block it found.
- `SeaAnemoneFeature::place` (149d60d10): from y 62 down to a solid floor above sea level - 15, in water a fan (1 in 4)
  or a coral plant.
Warm ocean region 99.8663 → 99.9954% identical; coral blocks 99.8/100 precision/recall, fans and plants 100/100.

## Dripstone residue: floor and deterministic (2026-10-09, worldgen/liq2)
`scripts/dripstone_floor.py` (with `examples/decorate_trace.rs` and scripts/data/dripstone_trace.patch) gives every
stable dripstone error an owner feature and judges each (source chunk, rule) sequence at its first desynced
feature: floor when a glow lichen or sculk difference (ours against BDS, or BDS against its `-run2` twin), a
dripstone cell the twin disagrees on, or an error left by an earlier-decorated floor sequence lies in that
feature's scan box (cluster: its columns, 12 up and down; pointed: 12 around at its height; large: 18).

| world | stable dripstone errors | floor | deterministic | (twin-unstable, not counted) |
|---|---|---|---|---|
| biome regions | 3266 | 2850 | 416 | 68 |
| tree regions | 5391 | 4006 | 1385 | 350 |
| structures | 11255 | 9105 | 2149 | 1081 |
| of which stronghold | 2793 | 2391 | 401 | |

Near origin, far out and the negative seed have no dripstone. The feat3 reading holds: about four fifths is the
floor, and it is one mechanism, column scans and snaps stopping at lichen that BDS spreads with a thread-local
generator (ours places fewer lichen, so ours scans on: ours-only base blocks outnumber BDS-only ones about 7 to 4).
The deterministic part is 30 to 83 desynced sequences per world, many of them first desyncs of `small_dripstone`
(snap to a wall, then a pointed spike).
- Fixed: waterlogged spikes (above); about 180 cells over all worlds, of which ancient city 110.
- Not it: the base list does not take stone's variants. BDS dripstone with tips under it inside our diorite or
  andesite blobs (cherry grove 5572,26,-7261) reads that way, but with granite, diorite and andesite replaceable
  every dripstone region got worse (structures about +6200, trees +3000, biomes +1800): those blobs come from
  later-phase neighbour chunks and land around BDS's dripstone.
- Open, first candidates from the split's ROOT list: tree regions' chunk 566,-388 (`small_dripstone` #1 at
  9054,44,-6193) and cherry grove's 346,-454 (cluster #2 at 5545,17,-7264), both phase 0 with no lichen
  difference in range. In cherry grove 346,-452 the first placing cluster matches BDS column for column inside
  the chunk; its mismatches are the columns in 346,-451, which that chunk's own clusters redo later.

## Left
- Sulfur springs (`random_sulfur_spring_with_tuff_feature`: structure templates) and the spring trail.
- Fossils (`fossil_feature`, desert/swamp 1 in 64 chunks) and the desert well: structure templates.
- Multi-direction lichen spreads and lichen/vein faces (thread-local in BDS): the oracle no longer grades face
  states of either (worldgen-bds-features.md, "Feature residue").
- Dripstone residue: world-state differences (lichen spreads, sculk) shift whole clusters' column draws (split
  above).
- Bubble columns by ticks (edge routing).

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
`OverworldGenerator::getGenerationHeight`. Unlabelled: 1465e8f20/1465e9af0/1465e9360 MultifaceSpreader
(toward direction, toward all, toward random), 148b68b30/148b68f50 `MultifaceBlock::{getStateForPlacement,hasFace}`,
148b41810/148b42070 sculk/vein `attemptUseCharge`, 148b42aa0 vein discharge, 148b40dd0/148b41640 `attemptSpreadVein`.
