# BDS template structures (static RE + probes + oracle, crates/sapling-worldgen/src/structure)

Companion to [worldgen-bds-structures.md](worldgen-bds-structures.md) (starts, pass order, mineshafts). Build
`bds-molang/bedrock_server.exe` (1.26.52.3); VAs absolute; labels at the end.

## Templates
- Files: the Java `.nbt` templates in `behavior_packs/vanilla*/structures/**` (newest pack wins; loose files,
  the brarchive entries are empty stubs). `scripts/templates_to_rust.py <bds>` transcribes the families in
  `FAMILIES` (igloo, shipwreck, ruin, ruined_portal, village, pillageroutpost, ancient_city, trial_chambers,
  trail_ruins, fossils, mansion) into `src/structure/template/templates.bin` (~4.6 MB, include_bytes): palette, templates
  (blocks in file order, jigsaw/data-marker/loot-chest specials), and a state table (template state -> palette
  block per clockwise rotation). Palette entries equal to a `Block` variant load as it; the rest are
  `Block::Template(index)` (one enum variant, names and states from the palette).
- Loader `LegacyStructureTemplate` load 141eedcf0: palette `Name` (minus `minecraft:`) through the Java-name map
  141f75940 (a few renames, else the Bedrock registry by name), every `Properties` pair through 141f71f20 (a hand
  switch over keys), `waterlogged=true` -> water in the second layer for blocks that take it. Blocks keep file
  order (placement order). Beds, flower pots and wall banners get synthesized block entity data.
- That converter is ~2000 lines of switch: **the conversion is measured, not ported**
  (`scripts/template_probe`). A crafted template holds every Java state of the families (one per even cell,
  three marker blocks giving the frame), swapped in for the igloo top templates of a private BDS copy; the
  script API reads each placed cell (`permutation.getAllStates()` adds legacy alias states: keep only the
  registry's keys, `registry.py` from acacia's blocks.bin). Only chunks within one of the start get blocks, so a
  second flat layout (x, z < 16) fills the rotations the first missed. Result: `scripts/data/template_blocks.tsv`
  (Java state, rotation, Bedrock name, states, waterlogged, flags). Brewing stands take their bottle bits from
  the block entity's raw item slots (Java 0-2 read as Bedrock 1-3), set by the generator. New families need a
  rerun for their missing states only (mkprobe's state list; the mansion's 1.11-era states took one run,
  `PROBE_SEED=20261003`, level type DEFAULT, `PROBE_REGISTRY` for parse_probe; lily pads mapped by hand).
  The pack's `scripts/data.js` is written per run: `CELLS` from mkprobe's states.tsv, `CANDIDATES` igloo starts
  in block coordinates (temple starts whose point biome is 12, 30 or 184); 30 igloos cover all four rotations.
  The abandoned camps' 262 states took one run (podzol and full water cauldrons mapped by hand).
- A second probe swaps all shipwreck templates (flooded placement) to list what BDS waterlogs:
  `scripts/data/waterloggable.txt` (stairs, slabs, fences, walls, panes, bars, doors, trapdoors, chests, ...;
  never double slabs).

## Placement (`placeInWorld` 141eeb8d0, settings ctor 141ef1240)
Settings: integrity 1, seed, flags (+8 terrain match, +9 mirror, +10 rotation, +0xb skip structure blocks,
+0xc skip jigsaws, +0xd flood), +0x10 ignored block, +0x18 chunk (INVALID = unbounded box), +0x48 wood remap,
+0x88 processors. Per block in file order:
1. skip the ignored block type (template pieces: air), structure blocks, jigsaws - before any draw;
2. integrity: `integrity < nextFloat()` on an MT seeded with the settings seed skips;
3. position: mirror flips template z, then the rotation turns about the origin (no pivot); box test;
4. block: rotation (141f70580; from the table), mirror (`template/mirror.rs`: flip the turned state across the
   world axis template z lands on), wood remap (doors, trapdoors, stairs, stripped logs special-cased; logs,
   slabs, double slabs, bark, fences, planks by list - `template/wood.rs`), processors;
5. flood: air below sea level becomes water, waterloggable blocks below sea level get water;
6. setBlock; a chest with block entity data and no `LootTableSeed` draws a loot seed from the piece's random.
After all blocks 141f5c5d0 updates shapes against neighbours. Saved worlds show fence/pane/bar connections and
stair corners following the *final* neighbours (also across chunks), so `template/connect.rs` sets them after
every chunk's pieces (with `rejoin_fences`): fences join their kind, gates and sturdy faces (full cubes, stair
backs); panes and bars join panes, bars, walls and sturdy faces; stairs take Java's `getStairsShape`, corners
named as Bedrock saves them; walls (`template/wall.rs`) take Java's `WallBlock` shape: sides join walls, panes,
bars, fence gates across them and sturdy faces, `tall` where the block above covers that side, post by Java's
`shouldRaisePost`. Hardcoded pieces' fences, bars and stairs go through the same pass (`rejoin_connections`).
Markers (`getMarkers` 141ee8850) come back in an MSVC `unordered_map<BlockPos, string>`: FNV-1a per coordinate,
boost-combined; list order = insertion order, a key whose bucket is taken goes after that bucket's last key
(`template/markers.rs`, no rehash below 9 keys). Order matters where marker handling draws.

## Who post-processes what
- Starts are drawn from the blueprint MT (`createBlueprints` 141ef9dc0: seed `zScale*z + xScale*x ^ levelSeed`,
  one draw skipped), but `getChunkPosInSpace` 141ef95f0 **reseeds the random it is given**: a start's draws
  continue from the last grid check of its `isFeatureChunk`. Ocean ruins and shipwrecks end with the monument
  test over chunks x-5..x+4, z-5..z+4: the generator is the monument grid MT of (x+4, z+4) after 4 draws.
- Post-process randoms are the chunk's structure MT (`levelSeed ^ (z*zf + x*xf)`), reseeded per feature.
- `ScatteredFeaturePiece` (igloo), shipwreck and ruined portal pieces post-process only in their start/corner
  chunk and place unclipped through the 3x3 block source; sapling gives a chunk every such start within one
  chunk, drawn with the start chunk's generator. `TemplateStructurePiece` (ocean ruins) post-processes in every
  chunk its box meets, clipped to that chunk, integrity MT seeded with that chunk's seed.
- Heights: BlockSource vt+0x178(x, z, includeWater) = `getAboveTopSolidBlock`: one above the highest solid,
  water counting only when asked.

## Igloo (scattered start in biomes 12, 30, 184; postProcess 14836fd90)
Origin (x*16, top(x*16, z*16, no water) - 1, z*16); rotation nextInt(4); nextFloat() < 0.5: top without trapdoor
only; else top with trapdoor, nextInt(8)+4 middles at rot(3,-1-3j,5) - rot(1,2,1), bottom at rot(3,-1-3n,5) -
rot(3,5,7); "chest", "Villager", "Zombie Villager" markers become air. Settings box y 0..319; ignored block anvil.

## Ocean ruins (start 148351e10, addPiece 148353240, piece 148354170/148354700, markers 148356170)
- Rotation nextInt(4); origin chunk corner + (0,0)/(15,0)/(15,15)/(0,15) by rotation; large = nextFloat() <= 0.3;
  warm (biomes 40-43): one piece, nextInt(4) big / nextInt(8) small; cold: index nextInt(4 big / 9 small, index 0
  = `ruin4`) and brick, cracked (0.7), mossy (0.5) pieces; integrity 0.9 big, 0.8 small. Large: nextFloat() <= 0.9
  adds a cluster: eight spots around the big box (each two draws, BDS's ranges), nextInt(4)+4 tries: pick a
  spot (nextInt(len), removed), rotation nextInt(4), kept if its 6x7 box misses the big 16x16.
- Per chunk: y from the heightmap at the origin (water counts), scan the footprint down through air/water; if
  the lowest floor is > 3 below and more than (width - 2) columns sink below height-3, y = lowest floor, else
  height - 1. Flood, ignore air. Archaeology: a capped rule (limit 5) collects placed gravel/sand cells, the
  finalize (145e12a20 via 141eedbb0) shuffles them with a Xoroshiro seeded by the template origin (low 32 bits
  of Java's `Mth.getSeed`, sign-extended) and makes the first five still-matching ones suspicious
  (`hanging=1`). Chest markers: marker cell -> sandstone (warm) / stone bricks (ruin3_brick) / gravel
  (ruin2_brick, big_ruin1_brick) / water; chest facing east below, waterlogged at y <= 63.

## Shipwrecks (start 1483719b0, piece postProcess 148372130)
Beached = biome 15/16/26 at the piece corner (y 63 or 31 by the centre's biome). Draws: rotation nextInt(4),
template nextInt(20) (beached: then nextInt(14); first 14 can beach), woods nextInt(9) (14a927de0: oak and dark
oak swapped). Origin as ocean ruins; y = mean top (no water) over the turned middle half of the footprint, -2
(templates < 14) or -4; beached: at least 63. Flood unless beached. After placement the loot chests' draws; then
markers in map order: marker -> water below 63 else air; treasure/supply chests: nextInt(10) < 5 removes the
chest (air above sea level, else water), else a loot draw; map chest: a loot draw. A version gate picks mean
(taken on 1.26 worlds) over minimum.

## Ruined portals (start 14835c7d0, postProcess 14835d070, ageing helpers 14835feb0/148360050/14835fc20)
Tags from the VanillaBiomeTags table (registry +0x98, 0x18 apart, init order 14393d890: ocean 0, desert 2,
jungle 3, extreme_hills 5, swamp 8, ice 17, mountain 18). Draws: placement nextFloat (>= 0.5 surface),
rotation, mirror (nextFloat >= 0.5), giant chance 0.05, template. Box from the corner by the turned (mirrored)
size, inclusive of +size. Surface: y = top(centre) - 1, desert half-buried (`y0 + nextInt(6) + h - y1`);
underground: from top - 2 - height down to base (70 in extreme hills/mountain, else -49), random between;
then lowered while fewer than 3 corners rest on solid (water counts off the sea floor; leaves never). Netherrack
spread (chance table 14a927a68, 29x29 around the centre), air pocket (ignore air on the surface unless
nextFloat < 0.5, never desert), flood when a box corner is water, then the netherrack below the floor, mossiness
(0.8 flooded, desert 0, jungle 0.8, swamp 0.5, else 0.2) and the ageing sweep (x, y, z): stone bricks and double
slabs (50%: cracked/mossy or random stairs), stone brick stairs (50%: slabs or mossy stairs), slabs and walls
(mossiness -> mossy, default states), netherrack (0.1 magma unless ice), obsidian (0.2 crying), lava (magma /
netherrack by sea floor and ice), gold (0.3: lava beside lava, else air).

## Pillager outposts (start 148357080, start postProcess 148358490)
Watchtower, then the overgrown watchtower (integrity 0.05), then up to four feature pieces from seven
templates around it. Each piece's y comes from five heightmap points of its own footprint; the start's
postProcess fills under each piece's bottom layer down through air and fluids with that block (cobblestone
where it is not a full cube).

## Woodland mansions (start 1480a8c20, placer 1480abea0, start postProcess 1480ab380; `src/structure/mansion`)
Java's 1.11 mansion on BDS's MT, with BDS's own details:
- Start: rotation nextInt(4); the lowest first-free height at local (7,7), (7,oz), (ox,7), (ox,oz) of the start
  chunk's generated terrain (ox/oz 12, or 2 where the rotation turns that way); origin (x*16+8, lowest+1,
  z*16+8). No y >= 60 test.
- Grid: Java's MansionGrid, but `from2DDataValue` is BDS's table (N, S, W, E; 14a7fe150), corridors 1480b72e0,
  rooms 1480b7a60 shuffled Sattolo-style (`nextInt(i)` for i from the last index down, never i itself).
- Pieces: `mansion/<name>` templates; BDS names drop Java's `_1` (carpet_south, carpet_west, indoors_wall,
  indoors_door) and add corridor_floor_2 above the first floor. Mirror 1 flips template z (Java LEFT_RIGHT),
  2 flips x (FRONT_BACK = LEFT_RIGHT a half turn further).
- Pieces are `TemplateStructurePiece`s (141f797b0): air placed, structure blocks skipped. Markers (1480b2360):
  Chest<Side> -> a chest facing the side turned by the piece (mirrors swap N/S resp. E/W), not over a chest;
  "Group of Allays" -> air; Warrior and Mage spawn only.
- Start postProcess: pieces, then cobblestone under the start box's bottom layer wherever a piece stands, then
  two pairs of stone stair runs out of the entrance (1480abb80), all stepping down through air and fluids no
  lower than y 2.

## Oracle (world 20261003-structures + run2, `blocks_match_bds_structures`)
| kind | precision (non-air) | recall |
|---|---|---|
| igloo, snowy slopes | 100% | 100% |
| igloo, snowy plains (basement) | 98.3% (98.9%) | 99.5% |
| ocean ruins | 99.2% | 92.6% |
| shipwreck | 99.8% | 62.9% (the rest is a dungeon and features) |
| ruined portal | 99.8% | 99.8% |
| pillager outpost | 98.0% | 89.9% (two feature pieces on uneven ground sit off by one) |
| woodland mansion | 99.5% | 89.9% (the rest is trees and moss carpets around it) |
Jigsaw structures: [worldgen-bds-jigsaw.md](worldgen-bds-jigsaw.md).
Remaining: iron bar/fence/wall joins to a few non-cube neighbours, water a basement cut open
(liquid stage).

## Labels
```
141eedcf0 LegacyStructureTemplate load       141f75940 Java name -> Bedrock block   141f71f20 Java property converter
141eeb8d0 LegacyStructureTemplate::placeInWorld 141f70580 rotate/mirror block     141ef1240 LegacyStructureSettings ctor
141f5c5d0 template shape update pass          141ee8850 template getMarkers          141eedbb0 processor finalize
145e12a20 capped processor finalize           141f797b0 TemplateStructurePiece::postProcess
14836fd90 igloo postProcess   148360c00 ScatteredFeaturePiece::updateAverageGroundHeight
148351e10 OceanRuinFeature::createStructureStart 148353240 addPiece 148354170 piece ctor 148354700 postProcess
148356170 ocean ruin markers  1483719b0 ShipwreckFeature::createStructureStart 148372130 shipwreck postProcess
14835c7d0 RuinedPortalFeature::createStructureStart 14835d070 portal postProcess 14835feb0 random stairs
148360050 netherrack column   14835fc20 portal gold block   1415f7a50 BlockSource::getAboveTopSolidBlock
14393d890 VanillaBiomeTags init  14837f180 template name globals (portals, igloo, shipwreck)
1480a83a0 VillageFeature::createStructureStart 141f79aa0 village start pieces 141ee6a70 JigsawPlacement::addPieces
141ee7000 jigsaw piece recursion 141ee7770 jigsaw candidates 14816cc00 village elements 1480bbe90 village pools
148357080 outpost start   148358490 outpost start postProcess
1480a8980 WoodlandMansionFeature::isFeatureChunk   1480a8c20 mansion start   1480abea0 createMansion
1480b72e0 mansion corridors   1480b7780 cleanEdges   1480b7a60 identifyRooms   1480b69c0 isRoomId
1480b3800 traverseOuterWalls   1480b3cc0 wall piece   1480b3eb0 createRoof   1480b1930/1a70 mansion piece ctor
1480b2360 mansion markers   1480ab380 mansion start postProcess   1480abb80 entrance stair run
```
