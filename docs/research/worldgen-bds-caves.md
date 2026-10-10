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
  (149e94de0) replaces open cells and spikes, stalagmites below the pre-worldgen heightmap.
- MT `nextGaussian` is the polar method in f32 with the second value cached in the generator (`+0x9d4`).
- A spike set into a water source keeps the water in the second layer (`LevelChunk::setBlock` 1409f59e0;
  `Block::into_water`, the `PointedDripstone...Wet` blocks). Every "air or water" test of these features reads
  the block's material and then the extra block's (world-gen API slot 0x30 = `BlockSource::getExtraBlock`
  1415f6880), so a waterlogged spike, lichen or plant is open: scans pass it, `growPointedDripstone` overwrites
  it, a cluster may start in it, `canPlacePool` counts it as water. (Until 2026-10-09 the port kept wet spikes
  closed to scans; opening them took 185 cells off the tree regions.) The lava tests read both layers too.
- No speleothem feature reads a heightmap except the large one's stalagmite cap, which is the pre-worldgen
  heightmap (worldgen-bds-features.md, "Heightmaps").

## Wrapper features (feature/cave.rs, multiface.rs, blob.rs)
- `snap_to_surface` (143bb7720, search 143bb7ac0): surfaces ceiling, floor, `random_horizontal` (enum 3: the
  ceiling or the floor, a horizontal surface, by one `nextBoolean`, true = ceiling) and `random_vertical` (enum 2:
  the walls, sides 2, 5, 3, 4 shuffled forward with three raw draws `% (i+1)`). The port had the two swapped until
  2026-10-09, so every `small_dripstone` snapped to walls and spent three draws where BDS spends one: found by
  decoding BDS's snap origins (breakpoint log) against the rule's draw stream, 7 draws apart instead of 9. From
  an open origin (air, or water by the block's material only) the scan walks allowed open cells (air if
  `allow_air_placement`, water if `allow_underwater_placement`) to a supporting face (or an
  `allowed_surface_blocks` block); from a solid origin it walks solid cells (`allow_non_air_placement`) to an open
  one. Every scanned cell must keep the origin's biome (`scanColumnWithinPosBiome`). Placement is beside the
  surface, or in it with `embed_in_surface` (offset `solid ^ embed`).
- "Supporting face" in snaps, vegetation patches and moss vegetation is `Block::canProvideSupport(face, 2)`,
  for multiface blocks `canProvideMultifaceSupport(face)`: see "Support" below. A snap that reaches a spike stops
  there and fails (live BDS, savanna plateau 9064,44,-6203).
- `multiface_feature` (143b83e50, attach 143b84880): origin air or water; the first face (of the pushed list:
  floor, ceiling, walls N E S W) whose neighbour `can_place_on` takes decides; success only when the cell changes,
  then one `nextFloat` (spread gate). Else each face's neighbour, retried `search_range` times with the same result.
  BDS shuffles faces with a thread-local generator and spreads with it too: the two oracle runs disagree on faces;
  sapling keeps the pushed order. That order also picks the neighbour cell when the origin itself takes no face,
  so which cell gets the lichen is thread-local too wherever two neighbour cells could (pale garden: ours under
  the ceiling at -7516,49,-4568, BDS's one cell west in both oracle runs and elsewhere in a third run).
  A spread (1465e9360, after `nextFloat < chance_of_spreading`) tries the six
  directions in thread-local order: glow lichen takes the first that can hold it (2026-10-10: BDS always
  spreads, and a lichen fewer shifts every lush patch around it, worldgen-bds-residue-lush.md); a vein spreads
  only when exactly one direction can ("Thread-local draws").
  Attach skips a face whose block lies in another chunk than the cell when that chunk is not decorated yet
  (`LevelChunk` +0xf0 in 2..5; live: 4 before decoration, 6 after) and the block is inside a stronghold's or a
  mineshaft's start box (HashedStrings 0x14ccbb900/930 = `minecraft:stronghold`, `minecraft:mineshaft`, asked of
  the world generator's `isStructureFeatureTypeAt` 14196ac20 -> 141efd3b0, which tests the starts' whole boxes).
  The next face is tried. Ported with the parity phases as "not decorated yet"; it moved two oracle cells.
  `can_place_on` holds plain names: the vein feature's list has the city's deepslate bricks and tiles, cobbled
  deepslate and `planks` (the six legacy woods), which the converter used to drop because only templates place
  them. Without them no vein feature landed on a city block (`Multiface::on_names`).
- Multiface spreading (feature/spreader.rs): `getSpreadFromFaceTowardDirection` 1465e8f20 tries same cell (face
  `to`), same plane (cell + `to`, face `from`), around the edge (cell + `from` + `to`, face opposite `to`);
  `getStateForPlacement` 148b68b30 adds a face only over a sturdy neighbour. Configs: lichen 14a8b32f0 (into air,
  own block, water source), vein 14a94cdb0 (not onto sculk or a catalyst; anything replaceable; non-veins are always
  valid sources). `spreadAll` walks faces up, down, N, S, W, E and the same order toward.

## Support (block/support.rs, generated block/support_table.rs)
`Block::canProvideSupport(face, type)` is inlined at every call site: the block's `minecraft:support` component
first (148a897e0; id 24 at run time, on the block, else on its type), else `BlockType` vtable 0x90. Faces are
the supporting block's (the one toward the asker; table 0x14a682118 turns a direction round); types are 0 centre,
1 edge, 2 any. The classes were read off a live server's registry (`scripts/dbg_blocks.py` ->
`scripts/data/block_types.tsv`, `scripts/support_data.py`), one per implementation:
- default 141a24e20: the solid-blocking property (`BlockType` +0x130 bit 0x40000), or bit 0x20000 for the top
  face. Lanterns, rails, torches, webs, doors, signs, carpets, bamboo, cactus, spikes, lichen: none.
- component shape 1 (stairs, 148a89b20): bottom or, upside down, top, and the back (`face == 5 - weirdo_direction`).
  Shape 0 (fences): top and bottom for centre and any.
- 146603600 slabs: top face of a top slab, bottom face of a bottom one (double slabs all); 1425e5d10 walls,
  panes, bars and 146608870 decorated pots: top and bottom, not for edge; 141a2cfe0/142fcec40 candles, chests,
  anvils, hanging signs: bottom, not for edge; 141a3b1c0 sculk sensors and shriekers, lecterns, farmland, grass
  path, campfires: bottom; 1425764f0 trapdoors: closed the face they lie in, open the side they stand on
  (table 0x14a7a2f68 by `direction`: 4, 5, 2, 3); 148b220d0 chains: ends of a vertical one; 141a45250
  cauldron: top, not for centre; 148b33d80 composter: all but top centre; 142fd2950 hopper: top; 140076010
  leaves, shulker boxes, buds, powder snow: never; 1400a2880 pistons: always; 142fce030 grindstone: the top of
  a standing one, the bottom of a hanging one (`attachment`), not for edge; 146602890 skulls: the bottom when
  `facing_direction` is 1, centre only; 148a32230 end rod and 148b5bff0 lightning rods: top and bottom of an
  upright one (`facing_direction` 0 or 1), not for edge; 148b51e20 shelves: the back face (table 0x4030502 by
  the cardinal direction's value), any type, so a vein hangs there. Piston arms and snow layers go by states
  sapling never meets and are left without support.
`canProvideMultifaceSupport` (vtable 0x98, default 141a24f10): a fence never, a stair by its shape, else centre
and edge support both. Overrides: leaves always (1400a2880: lichen and veins hang on leaves although nothing
stands on them), azaleas the top (142fd2950). Sculk's diagonal-move test and its shriekers call the default
141a24e50 past the vtable (`supports_multiface_plain`).
Vines (single-face 149d9a0f0, and the jungle's as far as fitted) take a solid-blocking block that blocks motion,
which leaves are. Before 2026-10-10 one predicate (`lush::sturdy`: collides and is not a listed fixture) served
all of these, so every template block with collision held veins: see "Sculk", the bias.

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
  the +-4/+-1/+-4 box around the cell above the cursor (148b4194f; Java's box, the port had it one lower)
  (`nextInt(50) < charge`, shrieker 1 in 11), else `nextInt(10)` decay by distance.
  Vein charge 148b42070: six faces shuffled inside-out (1403747f0), the first face on a replaceable block becomes
  sculk, veins around it lose faces on sculk (148b42aa0), veins spread from it; else `nextInt(5)` halves.
- The patch's walk is chaotic: BDS's sculk_vein_feature faces are thread-local, so whole patches diverge even on
  cells both BDS runs agree on.
- The bias (2026-10-10; ours had 18% more veins and 17% fewer sensors than either BDS run in the city region):
  not the spreader. A patch's positions come from the rule's own generator, so a phase-0 chunk run alone gives
  the same tries on both sides, and the first patch of city chunk -70,78 can be compared write for write
  (`scripts/dbg_sculk.py`: every `setBlock` call site of the patch, armed for the patch call only). Ours added
  a vein face toward a hanging soul lantern in the first write; with the support classes above 452 writes
  agree, up to a vein BDS's vein feature had spread there with its thread-local generator. The feature itself
  placed nothing in ours on that chunk (BDS: 6 veins, 6 spreads) until `can_place_on` kept its template names.
- Spreads of the vein feature: see "Thread-local draws" (still only a lone direction).

## Thread-local draws (2026-10-10, worldgen/sculk2)
The generator at `gs:[0x58]` +0xe60 wraps an MT at +0x460 that 1411889d0 seeds with four bytes from the system's
entropy source, per thread: no function of the seed. In decoration only `MultifaceFeature` draws from it
(callers of the spread 1465e9360: the attach 143b84880 and 1465e98c0, the in-game random spread; the six-face
shuffle 1403747f0 is also called by the vein behaviour 148b42070, with the rule's generator). It decides:
- the order of the feature's faces (one forward shuffle per call, and one more per searched neighbour that is
  thrown away): which face the block takes where several neighbours are `can_place_on`, and, when the origin
  takes none, which neighbour cell gets it;
- the direction of the spread (a six-way shuffle; the first direction that can hold one wins).
It does not decide whether a block is placed (only a cell that already holds the block with the chosen face can
fail), nor the spread gate (`nextFloat` of the rule's generator: 1.0 for veins, 0.5 for lichen), nor anything
in `SculkPatchFeature`, the spreader, the catalyst and shrieker rolls. So the count is fixed: every vein of the
feature comes with a second vein or face wherever a direction can hold one. Sapling keeps the pushed order
for faces and neighbour cells; for the spread see the end of this section.
Measured on phase-0 chunks run alone (`scripts/dbg_sculk.py`, `dbg_vein_cmp.py`, `dbg_sculk_cmp.py`): stony
peaks -228,260 (10 veins: 8 in our cell, 2 in another neighbour; BDS spread all 10) and badlands 594,-348
(4 veins: 3 and 1; 4 spreads). All 74 and 77 patch tries stand at our positions; 72 and 74 agree in every
write, and the five others agree for the first 130 to 982 writes, up to a vein the feature spread, and then
end with 0, 0, 0, 1 and 29 cells different, within 4 blocks of that vein, with the same number of conversions
(1003 and 1003, 1075 and 1075) and the later tries back in step. A differing vein moves a patch only around
itself while the patch's draw count holds; when it shifts (a vein's `nextInt(5)` is drawn only if no face
finds a block to convert), every later patch of the chunk walks differently, which is what the twin worlds
show as whole patches coming and going.
The spread's direction cannot be known, so the rule is the oracle's choice per family. Per decision with k
directions, none placed is one wrong cell and a guess 2(k-1)/k, before what the block does to later features.
Lichen takes the first unshuffled direction (worldgen/lush2: a missing lichen shifts lush patches and scans;
its own stable errors about double, the regions' totals fall). Veins spread only when one direction can: on
the merged tree the full oracle counts 35665 wrong + missing that way against 36393 with veins always
spreading (stony peaks +479, cherry grove +396, igloo +141, ancient city +92; trail ruins -329, jagged
peaks -139). Both are a guess: such totals swing by hundreds either way through single cells, and without
the spread sapling's deep dark holds one vein where every BDS run has two.
2026-10-10 (worldgen/edge3): the shuffles themselves are known and replayable against a one-worker run with
the generator seeded 0 (worldgen-bds-order.md, "The thread generator": every lichen of badlands on its cell);
"only `MultifaceFeature` draws from it" holds inside a chunk's decoration, not between chunks. A vein set into
a water source keeps the water and leaves it when removed (same file, "What the replay found").

A later-phase chunk and a city chunk, the same way: stony peaks -229,259 (phase 2) has all 86 patch tries at
our positions and all five veins in our cell (three of BDS's five spreads in the cell our first direction
names); its patches agree until the third placed one meets (-3658,6,4144), sculk in that run before the patch
got there and granite in ours: an earlier source's cell, inherited. Ancient city -70,76 has 36 tries at our
positions and the first placed patch alike in every write; the second went wrong after 18 writes at a stair
our structure stage had removed (worldgen-bds-jigsaw.md, "Elements and processors": the list predicate), and
the two after it from their first write, their draws shifted.
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
- Lush caves' hardcoded features (feature/lush.rs; decompiled 2026-10-09, vtables 14a98adf0, 14a928e30, 14a928e60,
  14a98ad70):
  - `moss_vegetation_feature` (149d99710): the weighted pick first (`nextInt(sum)`), then air at the position,
    support under it, the block's `mayPlace` (vtable 0x210); double plants get their upper half written at once.
  - `dripleaf_feature` (148299bc0): `nextFloat < 0.2` small (then `nextInt(4)` for its direction), else
    `nextInt(4)` picks `big_dripleaf_{south,west,north,east}_feature` (0, 1, 2, 3).
  - `vines_single_face_feature` (149d9a0f0): air only; sides north, east, south, west; a side holds a vine when it
    is solid-blocking, blocks motion and is not invisible bedrock.
  - `azalea_root_system_feature` (149d660d0): up the column from the position to the light heightmap (at most 184
    cells), the azalea tree at each cell over solid ground whose three cells are air or water (not all three
    water) until one grows; then per cell from one above the position up to the tree, 20 tries at
    `nextInt(3) - nextInt(3)` in x then z turn a block of list 0x14ccd2758 into rooted dirt (clay, moss block, sand,
    gravel, dirt, podzol, rooted dirt, grass block, mycelium, stone, deepslate, tuff, cave vines and their berry
    head, moss carpet, short grass, fern); then 20 hanging-root tries on a walk that never returns to the start
    (x, z by `nextInt(3)` pairs, then y by a `nextInt(2)` pair), each into air under a solid block. The port had
    guessed Java's shape: other rows, other list, draws in x, y, z from the start each time.
- `vegetation_patch_feature` (143bc8310) and `growing_plant_feature` (144c15200) match their decompiles (checked
  2026-10-09; a growing plant passes water of any depth where `allow_water`).

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

## Dripstone residue (2026-10-09, worldgen/liq2 then worldgen/drip)
`scripts/dripstone_floor.py` (trace from `examples/decorate_trace <seed> dripstone <regions>`) gives every stable
dripstone error an owner feature and judges each (source chunk, rule) sequence at its first desynced feature:
floor when a glow lichen or sculk difference (ours against BDS, or BDS against its `-run2` twin), a dripstone cell
the twin disagrees on, or an error left by an earlier-decorated floor sequence lies in that feature's scan box
(cluster: its columns, 12 up and down; snap: 3 around its column, 15 up and down; large: 18).

| world | stable dripstone errors liq2 -> drip | floor | deterministic (upper bound) |
|---|---|---|---|
| biome regions | 3266 -> 169 | 138 | 31 |
| tree regions | 5391 -> 519 | 465 | 54 |
| structures | 11255 -> 1819 | 1448 | 371 |
| of which stronghold | 2793 -> 868 | 815 | 53 |

Dripstone over all regions: precision 73.2 -> 96.4%, recall 69.9 -> 96.7% (badlands 98.0/99.3, cherry grove
99.6/99.9, ancient city 96.5/96.4, stronghold 90.4/90.3). Near origin, far out and the negative seed have no
dripstone. What moved it, in order of size:
- `random_horizontal` snaps (Wrapper features): biome regions 12531 -> 9286 wrong + missing, tree regions 10174 ->
  5022, structures 38393 -> 28703. liq2's split had put four fifths of these errors on the floor: a lichen
  difference lies in almost every cave feature's box, so the box test says nothing while the feature is wrong.
- Waterlogged spikes open to scans (Speleothems): tree regions -185; cherry grove 346,-452's clusters then match a
  live run in all 476 cluster, large and grow calls.
- Supports (spikes, then structure fixtures): about -30 and -200 (the second with growing plants passing water).
- Not it: the base list does not take stone's variants (granite, diorite, andesite replaceable made every region
  worse: those blobs come from later-phase neighbours and land around BDS's dripstone), and the pre-worldgen
  heightmap changed no cell.

What the rest is. Phase-0 roots of the split were run alone under the debugger (one-chunk regions,
`scripts/dbg_calls.py`). Of eleven in one batch, six match BDS call for call (594,-348; 562,-388; 240,-112;
-70,76; -58,-174; -84,-126), two diverge at a glow lichen of ours that the run did not have (at the snap origin
9573,26,-5535; under the ceiling at 9506,98,-5587), one at a snap that failed in that run and succeeds, with all
102 calls matching, when the chunk is run again alone (78,6: two BDS runs differ), one at stronghold terrain (BDS
has air at -1439,-3,-2010, one above our corridor; 9 such cells in the region) and one at a rail our snap took
for a floor (no draw involved; fixed).
Singly: -66,82 (ancient city) and the mansion's lush chunk -508,998 match in every call; meadow's 372,-510
diverges at a lichen BDS spread to 5965,23,-8150 in that run and in the oracle's twin and to the next cell in the
oracle's main run. So the "deterministic" column is run-dependent as far as it was sampled: a phase-0 chunk
decorated alone sees bare terrain, as in our model, but in the oracle's run the same-phase chunks two away were
decorated first (BDS works from the outside of the loaded area inwards, worldgen-bds-debugger.md) and had written
into the chunks between; the rest is where the thread-local generator put lichen.

Lush caves (same method, snap, patch, accepted cell and growing plant logged): the pale garden's phase-0 chunk
-470,-286 matches a live run for 1158 calls, then a moss ceiling patch (at -7515,47,-4564) takes one cell fewer
than BDS because our lichen hangs under the ceiling at -7516,49,-4568 (ours tried the cell above the rule's
position first; BDS's neighbour order is thread-local and both oracle runs took the cell to the west). The
mansion's -508,998 matches in all 997 calls although the oracle has 420 lush cells wrong in that chunk: run
order again.

## Left
- Sulfur springs (`random_sulfur_spring_with_tuff_feature`: structure templates) and the spring trail.
- Fossils (`fossil_feature`, desert/swamp 1 in 64 chunks) and the desert well: structure templates.
- Multi-direction lichen spreads and lichen/vein faces (thread-local in BDS): the oracle no longer grades face
  states of either (worldgen-bds-features.md, "Feature residue").
- Dripstone and lush residue: no port difference is known; what is left follows the lichen's thread-local cell
  and BDS's same-phase decoration order (above). The order could be read off a run (`scripts/dbg_order.py`) and
  given to the oracle test to shrink that floor; the generator cannot follow it.
- Moss vegetation and the small dripleaf: their blocks' `mayPlace` (vtable 0x210: in the world's height, the cell
  buildable over, then `mayPlaceOn` 0x220 of the block below) was read and agrees with plants.rs: azalea on clay
  or the dirt list (145810140), grasses on the dirt list with no liquid in the cell (1465ec830/14715cec0), moss
  carpet over anything but air (141a3eeb0), small dripleaf on clay or moss, or in water on the dirt list
  (1466059d0). The dirt list 0x14ccd3328, read live: dirt, grass block, podzol, coarse dirt, mycelium, rooted
  dirt, moss, pale moss, mud, muddy mangrove roots, farmland.
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
