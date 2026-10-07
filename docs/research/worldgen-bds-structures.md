# BDS overworld structures (static RE + oracle, crates/sapling-worldgen/src/structure)

Companion to [worldgen-bds.md](worldgen-bds.md). Build `bds-molang/bedrock_server.exe` (1.26.52.3, PE `6ab54bee`),
bdsre scratch copy; VAs absolute. Labels at the end.

## Two systems
- **Legacy `StructureFeature`s**, hardcoded C++: the overworld registry (`VanillaDimensionFactory::
  createOverworldGenerator` 148810880) adds, in this order: village, stronghold, scattered (`RandomScatteredLarge
  Feature`: desert/jungle pyramids, swamp huts, igloos), mineshaft, mansion, buried treasure, pillager outpost,
  monument, ocean ruins, shipwreck, ruined portal, ancient city. Vtable: dtor, shouldAddHardcodedSpawnAreas,
  shouldPostProcessMobs, getRequiredBiomes, getNearestGeneratedFeature, **isFeatureChunk (slot 5)**,
  **createStructureStart (slot 6)**, getStructureAt. Village, outpost and ancient city build their pieces with BDS's
  own legacy jigsaw (`VanillaVillageJigsawStructureElements`, pool paths as exe strings) over Java-format `.nbt`
  templates; shipwrecks, ocean ruins, ruined portals, igloos and mansions place `.nbt` templates; mineshafts,
  strongholds, monuments, pyramids, swamp huts and buried treasure are pure code.
- **v2 jigsaw (`br::worldgen`)**: structure sets with `RandomSpreadPlacement` (144cfea60). From pack JSON
  (`behavior_packs/vanilla_1.19.80/__brarchive/worldgen/*`): trail ruins; `vanilla_1.26.50`: abandoned camp (18
  biome variants, 37/8, salt 91231127, not ported). Trial chambers are a C++ v2 set (no JSON; neither the salt nor
  any spacing appears as an immediate). Starts are made in `loadChunk` (`preProcessStructures` 14196b7b0 →
  `_tryGenerateStructure` 14196cc20; several structures per set pick by Java `setLargeFeatureSeed` weights).
- Data inventory (pack archives, 1300 `.nbt` templates in Java NBT with Java block names, 15 `.mcstructure`): the
  research notes `st/data/NOTES.md` in the session scratchpad; templates are not ported.

## Start positions
Grid kinds: region `r = floorDiv(chunk, spacing)`, MT19937 seeded with the u32
`r.x*0x9939F508 + r.z*0xF1565BD5 + levelSeed + salt` (Java's region constants truncated; only the seed's low 32
bits matter), `range = spacing - separation`, x then z, each `nextInt(range)` (linear) or the mean of two
(triangular; draw order x1 x2 z1 z2). `StructureFeature::getChunkPosInSpace` 141ef95f0, inlined in most checks.
Biome checks use `BiomeSource3d::containsOnly` 14805f710: every block on the 4-step lattice within `r` of the
centre, x outer, z, then y clamped to the world, has an allowed biome. A point biome (`_getBiome` 14805f850 →
1481df3b0) is the climate of the block's quart column with depth at the block's own y, no lattice, no warm start.
"surface" below is `getPreliminarySurfaceLevel` at the chunk's corner quart (0x7fff where none).

| Kind | spacing/sep, salt, spread | Check after the grid (`isFeatureChunk`) |
|---|---|---|
| village | 34/8, 10387312, tri (27/10 in old worlds) | containsOnly(centre at surface, r 2, biomes with `village_type`: 1 2 5 12 19 30 31 35 129 186) |
| stronghold | see below | in the spiral list, or the 200x200 grid start |
| scattered | 32/8, 14357617, linear | point biome at centre/surface in 2 17 21 22 6 134 12 30 184; min of the chunk's raw column heights >= 63 (`computeChunkHeightMap` 14196d010: one above the highest non-air block of `_prepareHeights`, water counts) |
| mineshaft | every chunk | MT(levelSeed): a, b = two `nextInt()`; MT((b*z ^ a*x) ^ levelSeed); skip 1; `nextFloat() < 0.004` and `nextInt(80) < max(abs x, abs z)` (both drawn) |
| mansion | 80/20, 10387319, tri | containsOnly(r 32: 193 29 157 188 187 194) |
| buried treasure | 4/2, 16842397, tri | containsOnly(r 3: 16 26 25 15) |
| outpost | 80/24, 165745296, tri | point biome in 1 129 35 12 5 19 30 31 2 186 183 182 189 184 192 185 (no village test) |
| monument | 32/5, 10387313, tri | containsOnly(r 16: deep oceans 24 45 43 41 47) and (r 29: + 0 44 46 42 40 11 7) |
| ocean ruins | 20/8, 14357621, linear (12/7 tri old) | point biome ocean (0 24 45 47 43 44 46 42 40 41); no monument start in chunks x-5..x+4, z-5..z+4 |
| shipwreck | 24/4, 165745295, linear (10/5 tri old) | point biome in 16 26 0 44 42 46 15 24 45 43 47 40; containsOnly(r 20, or 10 on beaches 15 16 26, the centre's own biome); no monument nearby |
| ruined portal | 40/15, 40552231, linear | none |
| ancient city | 24/8, 20083232, tri | biome at (x*16, -27, z*16) is deep dark (190) |
| trial chambers | 34/12, 94251327, linear, **Java LCG** | v2: biome at the start piece's centre (not ported: we test the chunk centre at y -30, not deep dark) |
| trail ruins | 34/8, 83469867, linear, **Java LCG** | v2 `has_structure_trail_ruins` (155 30 21 32 160 5) at the start piece (approximated at centre, surface-15) |

v2 grid: Java `Random(rx*341873128712 + rz*132897987541 + seed64 + salt)`, `nextInt(range)` x then z (144cfea60).
Stronghold (148373d00, list 1483740e0): MT(levelSeed), angle `nextFloat()*2pi`, radius `nextInt(16)+40` chunks;
up to 1000 steps: scan chunks x-8..x+7, z-8..z+7 around (floor(cos*r), floor(r*sin)) (Mth table) for a village
start; a hit is a stronghold at that village chunk, then +0.6pi and +8, a miss +pi/4 and +4; stop at 3. Plus,
beyond 10 chunks of the origin, one per 200x200 cell: MT(gz_c*-0x41975147 + gx_c*-0x645414af + levelSeed +
0x5d534e7) for the cell centre g*200+100, dx = raw%100, dz = `nextInt(100)`, kept when `nextFloat() < 0.25`, at
g*200+50+d.

`/locate structure` (`findNearestFeaturePositionBySpacing` 141efb740): rings of regions around the origin's
region (100 rings), each ring's boundary x-major; the first start found wins, not the nearest.

### Validation
`/locate` from BDS's console needs no chunks: `execute positioned X 64 Z run locate structure <kind>` (script
`locate.sh` in the scratchpad: fresh world, own ports, one answer line per command, `list` as end marker).
3772 locates for seed 20261003 (grid every 1024/2048 blocks over +-8192, and 1.6M out). `examples/locate_check`:
every legacy answer is one of our starts (2712/2712 + 314 mineshafts + 106 strongholds); our ported search gives
BDS's answer in 2706/2712; the six misses are villages on plains near rivers that BDS rejects and our point
biomes accept (cause not found: same surface, same lattice biomes). v2: answers are the start piece position, a
chunk off; trial chambers 310/314 near a start of ours (296/314 searches agree), trail ruins 271/314 (212/314):
their biome test needs the jigsaw start piece. Unit tests: `structure::tests`.

## Pass order and post-process
Chunk tasks (1409ddfc0): Gen (`loadChunk`: terrain, surface, carvers) → **Structure PP** → Chunk PP (lava lakes,
feature rules) → CFRD → Relight. Structures are placed before features. `structurePostProcessChunk`
148213a20 → `postProcessStructureFeatures` 14196b2c0: per feature in registry order, the chunk MT reseeded with
the large-feature chunk seed (`levelSeed ^ (cx*a + cz*b)`, a/b forced odd), then each cached start whose box meets
the chunk (`foreachIntersectingStructureStart`, hash-map order), `StructureStart::postProcess` 141f60830: every
piece whose box meets the chunk, sharing the generator; failed pieces are not dropped. Starts and their pieces
are made once (`createBlueprints` 141ef9dc0: per chunk an MT seeded `xScale*x + zScale*z ^ levelSeed`, one draw
skipped, `isFeatureChunk`, then `createStructureStart` on the same generator) and cached.
Piece writes go through `placeBlock` (141f59f90: clipped to the chunk, `canBeReplaced`), but some code calls
`BlockSource::setBlock` directly (unclipped, reaching the 3x3 view) and reads neighbours unclipped.

## Mineshafts (src/structure/mineshaft, 14831b270 / factory 14831bf70)
- BDS's port of Java's legacy mineshaft; no random orientation for the room (world coordinates), pieces SOUTH 0,
  WEST 1, NORTH 2, EAST 3; `getWorldX/Z` as Java's.
- createStructureStart continues the chance generator: room box `(x+2, 50, z+2)..(x+9+dx, 54+dy, z+9+dz)` with
  the draws in **C++ argument order: dz, dy, dx**. Mesa (point biome at (x*16, 0, z*16) tagged mesa: 37-39,
  165-167) uses dark oak and room children only when `nextFloat() <= 0.5`. Height (new worlds): `j = -64 + ySpan
  + 1`, plus `nextInt(53 - j)` when j < 53, shift by `j - maxY`; mesa: `63 + nextInt(surface - 62)` (when the
  surface quart at the box centre is above 63) minus the box's middle.
- Factory: depth <= 8, within 80 blocks of the room's corner; `nextInt(100)`: >= 80 crossing (`nextInt(4) == 0`
  two floors), >= 70 stairs, else corridor of 4..2 sections (`nextInt(3)+2`, longest without collision), rails
  `nextInt(3) == 0`, spider `!rails && nextInt(23) == 0`. Children as Java's (crossing upper exits on
  `nextBoolean`).
- `isInInvalidLocation` (14831dd40): the box grown by 1 and clipped to the chunk has deep dark (chunk biomes) at
  its centre, or water/lava on a face (141f59c80).
- Corridor (14831de50): air box, `generateMaybeBox` 0.8 (a float per cell, all cells), spider webs 0.6 interior
  only; per section: support when every block over its span at y 3 is not air (unclipped), fences, then
  `nextInt(4)` planks at both ends or across plus torches (`nextFloat() < 0.05` each, facing by orientation);
  eight cobwebs `(chance, x, z)` that test the interior at (x, 3, z) but place at (x, 2, z-1)
  (`maybeGenerateBlockIfNotFloating`: float < chance, two non-air neighbours in the chunk); the spawner
  `nextInt(3)`. Then floors: planks where the block below is air or chain and the corridor floor is under the
  heightmap (unclipped setBlock); log pillars down (10) or fence + chains up (20) under floor planks at z 2 and
  len-3; rails over full blocks, `nextFloat() < 0.7` interior else 0.9. Crossings and stairs: Java's boxes, floors
  like corridors. Room: dirt floor under existing blocks, air up to 3, entrances, upper half ellipsoid (f32).
- Bedrock 1.26 fences save `connection_*` states that follow their final neighbours, across chunk borders:
  sapling sets them after the chunk's pieces from its neighbours' pieces (`rejoin_connections`).

Buried treasure (148312c40): one chest (north) at (x*16+8, z*16+8): from 4 or 5 (`nextInt(2)`) below the highest
solid block (fluids passed) down until enclosed by solid blocks above and around, then down to solid ground.

## Scattered (src/structure/scattered, 14835a7b0)
Biome at the centre picks desert pyramid (2, 17; 21x15x21), jungle temple (21, 22; 12x10x15), swamp hut (6, 134;
7x7x9) or igloo (12, 30, 184; template, not here), at (x*16, 64, z*16), orientation always 0 (south).
- Height is fixed in the first chunk BDS post-processes the piece in (we assume the start chunk) and cached.
  Pyramid (148360db0): draws `nextInt(3)` on every postProcess; height = min floor height over the box columns in
  that chunk minus that chunk's draw. Hut and jungle temple: `updateAverageGroundHeight` (148360c00) = mean of the
  top (fluids count) over the box columns in the chunk, plus 1 (hut) or 0.
- Pyramid cellar (1.20 suspicious sand): Java Xoroshiro on `Mth.getSeed` of world (16, -4, 13) truncated to i32,
  `std::shuffle` (j = `nextInt(i+1)`); a loot draw per suspicious sand only when its chunk is the current one or
  was processed earlier (z-major assumed); stair sand `nextBoolean` on the chunk generator. (1, 1, 2) and
  (W-2, 1, 2) are unturned east stairs.
- `createChest` (141f49ef0) / `createDispenser` (141f4a850): set only inside the chunk and where no chest is, then
  one `nextIntUnbounded`. `getOrientationData` (141f4a1d0) leaves chests alone (data 2 N, 3 S, 4 W, 5 E as given).
- Jungle selector (148321b40): `nextFloat() < 0.4` cobblestone else mossy, a draw per cell.

`getOrientationData` per state (orientation 0 S, 1 W, 2 N, 3 E; 2 = identity): weirdo_direction 0..3: S [0,1,3,2],
W [2,3,0,1], E [2,3,1,0]; facing_direction 2..5: S [3,2,5,4], W [4,5,3,2], E [5,4,2,3]; direction 0..3: S [2,3,0,1],
W [3,0,1,2], E [1,2,3,0]. Door direction 0..3 saves as cardinal south, west, north, east. Torch data 1..5 = west,
east, north, south, top; pieces inline it (Java east wall torch: [1,3,1,3] by orientation, west [2,4,2,4],
south [4,1,3,2], north [3,2,4,1]). Bedrock 1.26 saves stair corners, fence and iron bar connections from the final
neighbours (`rejoin_connections`).

## Monuments (src/structure/monument, 148332c00, MonumentBuilding 148334260)
MT(levelSeed): a, b = two `nextFloat()`; start seed `(a*cx) as i64 as u32 ^ (b*cz) ^ levelSeed`; facing
`nextInt(4)+2` (Bedrock facing 2 N, 3 S, 4 W, 5 E). Java 1.18's room graph, fitters and pieces, except: the room
shuffle uses a fresh MT on the start seed (`nextInt(i)`, i from n-1 down); no 58x58 water clearing box; water boxes
write water below y 63 and air above (no keep); wing design `nextIntUnbounded() & 1`, and design 0 lacks Java's
(15,0,21)-(16,4,21) box; chunk tests for the lower back (0,51)-(57,57) and middle left (7,21)-(13,54).

## Strongholds (src/structure/stronghold, 148375450)
- Start on the blueprint generator (`a*x + b*z ^ levelSeed`, a/b the mineshaft's two `nextInt()`, one draw skipped):
  stairs down (x*16+2, 64, z*16+2)-(+4, 74, +4), orientation `nextInt(4)`, imposes a five-way crossing.
- Weights in BDS order (weight, max, least depth): straight 40, prison 5/5, left 20, right 20, room 10/6, straight
  stairs 5/5, stairs 5/5, five-way 5/4, chest corridor 5/4, library 10/2/5, portal room 10/1/6.
- `generatePieceFromSmallDoor` (14809b270): depth <= 50, within 112 of the start's corner; works on a **copy** of
  the generator (siblings draw the same numbers; only the pending pick `nextInt(len)` advances the start's
  generator); imposed piece first; 5 tries of `genrand % total`: break when exhausted, too shallow, or the previous
  kind (if more than one weight left); else filler (`findPieceBox`, y > 1). `createPiece` (148099bd0): Java's
  boxes, y0 > 10, no collision; door `nextInt(5)` (2 wood, 3 grates, 4 iron); straight two `nextInt(2)==0`; room
  `nextInt(5)`; five-way `nextBoolean` x3 then `nextInt(3) > 0`; library tall if 11 high fits.
- No portal room: the last piece is dropped and a portal room put beside the piece before it (14809c9e0), unless
  that one is below y 11. Then `j = span - 62` (+`nextInt(53 - j)` below 53), shift by `j - maxY`.
- Pieces as Java's, but selector boxes fill air too and draw for every edge cell (148099b60: < 0.2 cracked, < 0.5
  mossy, < 0.55 infested); slabs are stone brick slabs; the prison hall has no iron doors; the storeroom torch is
  the default state (facing unknown); chests west (corridor, library, storeroom) and north (tall library).
- Terrain buries the pieces: `setBury` per piece box (144d065e0, via 144cd0700); `_prepareHeights` adds, after the
  noodle caves and before the aquifer, `clamp(1 - sqrt(dx² + (dy/2)² + dz²)/6)` per box (dx, dz outside the box,
  dy from its floor, |dy| < 12) from a f32 table (144cd3d40, summed in f64 by 144cd50c0): src/structure/bury.rs.
  Beards (villages, outposts) and v2 adaptations are not ported.

## Terrain adaptation
`FeatureTerrainAdjustments::setBeardifier/setBeardAndShaver/setBury`: kinds 1 bury (dx, dz <= 5), 2-4 beard
kernels (x0.8). Of the hardcoded structures only strongholds register (bury, above); jigsaw ones beard.

Woodland mansion (1480a8c20): Java's room grid in code, then `.nbt` templates for every room; left to the
template port.

## Oracle (2026-10-07)
New world `20261003-structures` (+`-run2`): 807 chunks in 16 regions around one start of each kind
(`blocks_match_bds_structures`). Structures tally (tests/oracle_support/structures.rs): placed = cells the
structure stage changed (credited to ported kinds), wanted = BDS structure blocks in a kind's rough area.
Mineshaft non-air precision 98.6% near origin, 99.2% far out, 100% badlands (dark oak), 90-97% swamps; the far-out
trial chamber shows as 5710 wanted cells (tuff bricks, waxed copper, chiseled tuff). Buried treasure exact (4/4).
Precision (non-air) / recall: desert pyramid 100/99.7%, swamp hut 100/99.2%, jungle temple 99.9/78% (misses are
vines and features), monument 98.4/99.8% (water against kelp), stronghold 96.5/71% (the rest under trial chambers,
a mineshaft and village blocks).
Known gaps: eroded badlands' mesa mineshafts (BDS places nothing there; ours a room at y 93: the height getter
(createStructureStart's param_7) gives the same value as ours elsewhere; unresolved), rails and webs desync in a
few corridors (frozen ocean 79% non-air), cross-chunk reads use the neighbours' carved terrain; plains villages
near rivers (the six /locate misses) not looked into.

## Prior art and licences
Bedrock finders (scratchpad `st/prior-art.md`): bedrock-dev/MCBEStructureFinder (MIT, pre-1.18), stirante/
featurelab (MIT, RNG), unworthyzeus/minecraft-seed-finder (Apache, JS); Nel-S/ChunkBiomes (Chunkbase-derived,
reference only), maehy/MCBE-1.18-Seed-Finder and forks (no licence, stronghold), cubiomes (MIT, Java only). They
agree on the region seed and the legacy table; all values above were re-derived from BDS and checked with
/locate. None has Bedrock mineshaft pieces or biome checks.

## Labels (bdsre-struct `names.user.json`)
```
141ef95f0 StructureFeature::getChunkPosInSpace      141efb740 StructureFeature::findNearestFeaturePositionBySpacing
141ef9dc0 StructureFeature::createBlueprints        141f60830 StructureStart::postProcess
148810880 VanillaDimensionFactory::createOverworldGenerator  14196d010 WorldGenerator::computeChunkHeightMap
14196cc20 WorldGenerator::_tryGenerateStructure     144cfea60 br::worldgen::RandomSpreadPlacement::getPotentialStructureChunk
1459409e0 JavaRandom::nextInt                       148210f70 OverworldGeneratorMultinoise::prepareHeights
14805f710 BiomeSource3d::containsOnly               14805f850 BiomeSource3d::_getBiome
148264190 BiomeSource3d::_getBiomeCached            1481df3b0 OverworldNoises3d::sampleTargetPoint
1480a80e0/1480a7d70 VillageFeature::{isFeatureChunk,VillageFeature}
148373d00/148373bb0/1483740e0/1483745b0 StrongholdFeature::{isFeatureChunk,StrongholdFeature,generatePositions,getNearestGeneratedFeature}
14835a4e0/14835a0c0/14835a7b0 RandomScatteredLargeFeature::{isFeatureChunk,ctor,createStructureStart}
1483608a0 ScatteredFeaturePiece::ScatteredFeaturePiece   14836de70 SwamplandHut::postProcess
14831b070/14831b270 MineshaftFeature::{isFeatureChunk,createStructureStart}
1480a8980/1480a86e0 WoodlandMansionFeature::{isFeatureChunk,ctor}
148312270/148311fe0/148312510 BuriedTreasureFeature::{isFeatureChunk,ctor,createStructureStart}  148312c40 BuriedTreasurePiece::postProcess
148356de0/1483567a0 PillagerOutpostFeature   1483328f0/148332280 OceanMonumentFeature   148351c30/1483516a0 OceanRuinFeature
148371610/148370fe0 ShipwreckFeature   14835c660 RuinedPortalFeature::isFeatureChunk   148311580/148311370 AncientCityFeature
14831bf70 MineshaftPiece::generateAndAddPiece       14831dd40 MineshaftPiece::isInInvalidLocation   14831c830 ::canBeReplaced
14831ca70/14831d010 MineshaftRoom::{addChildren,postProcess}   14831d360/14831de50/14831da00 MineshaftCorridor::{addChildren,postProcess,postProcessMobsAt}
14831d770/148320130 MineshaftCorridor::{_placeCobWeb,_fillPillarDownOrChainUp}
1483205e0/148320950 MineshaftCrossing   148321790/148321870 MineshaftStairs
141f59f30/141f59f60/141f59f90/141f5a1b0 StructurePiece::{getWorldX,getWorldZ,placeBlock,generateBox}
141f5aa60/141f5aff0/141f5b3d0/141f59c80/141f5b7e0/141f4a1d0 StructurePiece::{generateMaybeBox,
  maybeGenerateBlockIfNotFloating,generateUpperHalfSphere,isInInvalidLocation,isReplaceableByStructures,getOrientationData}
1409ddfc0 ChunkSource::_launchNextTask
148360db0/148366d30/14836f380 {DesertPyramidPiece,JunglePyramidPiece}::postProcess, SwamplandHut::postProcessMobsAt
148360c00 ScatteredFeaturePiece::updateAverageGroundHeight   141f49ef0/141f4a850 StructurePiece::{createChest,createDispenser}
148332c00/148375450/1480a8c20 {OceanMonument,Stronghold,WoodlandMansion}Feature::createStructureStart
148337c70/148340f10 OceanMonumentBuilding::{postProcess,generateWing}; room pieces 148342020..14834fb70 (entry, simple,
  simple top, double y/x/z/xy/yz, core, wing, penthouse)
14809b270 StrongholdPieces::generatePieceFromSmallDoor   148099bd0 ::createPiece   14809c9e0 ::placePortalRoomFallback
14809bdf0 StrongholdPiece::generateSmallDoor   14809c840/14809c940 ::generateSmallDoorChild{Forward,LeftRight}
148099b60 SmoothStoneSelector::next   Stronghold* postProcess: 14809d0f0 stairs, 14809d650 chest, 14809dbe0 filler,
  14809e1f0 five-way, 14809ef60/14809f220 turns, 14809f4f0 library, 1480a1b20 portal, 1480a4b10 prison,
  1480a50f0 room, 1480a6f90 straight, 1480a76b0 straight stairs
144cd0700 FeatureTerrainAdjustments::setBury   144cd50c0 ::calculateContribution   144cd3800/144cd3d40 ::init{Beard,Bury}Kernel
14822eb60 OverworldGeneratorMultinoise::_prepareHeights   149e9dae0 MonsterRoomFeature::place   149e923b0 LakeFeature::place
```
