# Structure spawns (BDS 1.26)

How BDS spawns a structure's own mobs (fortress, ocean monument, swamp hut, pillager outpost) and
what sapling copies: the saved spawn areas (sapling-leveldb `spawn_areas.rs`), the boxes generation
files (sapling-worldgen `structure/spawn_areas.rs`), and the spawner's pass (sapling-shard
`spawning/structures.rs`). [D] = decompiled (bdsre, bds-blocks exe), [S] = read off BDS saves,
[L] = drilled live, [U] = unverified.

## The saved record [S]

Chunk tag 0x77, one per column a spawning or data-driven structure reaches (seed 20261003, 43
records over a hut, an outpost, a monument, trial chambers, abandoned camps and a fortress):
`u32 1`; names (`u32` count, then `u32` index, `u16` length, bytes); boxes (`u32` count, then per area
from the highest id down `u32` id, six `i32`); data-driven areas (`u32` count, then `u32` id, `u32`
name, `u32` whole); hardcoded areas (`u32` count, then `u32` id, `u32` name, `i32` y offset, `u32`
whole). Id 1 is the start's whole box, the rest its pieces meeting the column, all cut to the
column's x and z. BDS reads the record back; it recomputes it (`14196d3b0`) only for chunks saved
before version 10. Test: `bds_records_read_and_write_back_byte_for_byte`.

| structure | y offset | whole box | pieces |
| --- | --- | --- | --- |
| swamp_hut | -3 | the start box, unmoved (y 64) | the placed hut |
| pillager_outpost | -3 | the pieces' columns, y 63 to 319 | each placed piece; tower and overgrowth twice |
| monument | 0 | the building | the building |
| fortress | 0 | the start box | the pieces meeting the column |
| igloo | 0 | the top's unturned box at y 64 | the same box one below its average ground (Java 1.12's `offsetToAverageGroundLevel(-1)`), not where the template stands |
| desert_pyramid, jungle_pyramid | 0 | the start box, unmoved (y 64) | the placed pyramid |
| trial_chambers, trail_ruins, abandoned_camp_<variant> | (data-driven) | the pieces' union | each piece meeting the column |

Sapling files these in the column's own post-process (`spawn_areas::file`) and saves them with
generated columns (`StoredColumn::spawn_areas`); test `spawn_area_tests.rs` against the records.
No block changes: `tools/replay-all.sh` 0 wrong, 0 missing (2026-11-05).
Whole-only columns of an outpost (its box meets them, no piece does) keep a record of the whole
box alone, as BDS's (research/captures/bds-jxoutpost.txt's world: chunks (116, -73) and (117, -73));
test `outpost_columns_no_piece_meets_keep_the_whole_box`. Outpost pieces past the start chunk find their height by the run's post-process order, as
their blocks do.
Data-driven and the other hardcoded structures, read 2026-11-05 off the oracle world
~/oracle-worlds/20261003-structures (`records <world> tag=119`, 2282 areas): a data-driven structure
files its whole box in every column it meets, a piece or not (trial chambers' y -45 to 15 alone in
chunk (3, 17)); a camp under its variant's id (`minecraft:abandoned_camp_meadow`); data-driven areas
come before hardcoded ones in a column (chunk (81, 6): trial chambers ids 1 to 6, the jungle pyramid
8 and 9). Settled 2026-11-08 (thirteenth3au): a name and each area take the next of one count from 0,
so the trial chambers' name is 0, the jungle pyramid's 7; names are listed from the highest index
down, as the boxes. Sapling now writes so (`encode_spawn_areas`, test
`a_second_name_takes_an_id_of_the_areas_count`, the oracle record byte for byte). Sapling files them so (`data_driven.rs` `file_v2`, first
in the post-process; `igloo.rs`; `stage.rs`); tests in `spawn_area_tests.rs`. `tools/replay-all.sh`
still 0 wrong, 0 missing.

## The spawner [D]

`BedrockSpawner::tick` (1415eebf0), after the attempt's column:

- Natural clusters: at each level whose floor block lies in a piece of a structure with a list
  (1438cfcd0), the cluster is skipped. All four register for pieces only (filter flag 1, 141f82140).
- The pass: the ticked chunk's pieces of such structures give their middles (1438db9d0): x0 plus
  half the width, rounded down, z alike, y the piece's top plus the y offset. `max(2, n) / 2` tries,
  each `nextInt(n)` a middle, `findNextSpawnBlockUnder` down from it (water only when the column's
  first spot is under water), the pieces at the found floor asked again (1438cf5c0, then 1438cfcd0),
  an entry by weight (all 1), `canSpawnInConditions`, a herd of `min + roundf(f² (max - min))`, then
  `isSpawnPositionOk`, the caps ("The lists and caps"), the mobs at the floor's
  corner plus one up (`1415efb20`: integer x and z, no inset). A refused spot ends the pass.
- The outpost's whole box y 63 to 319 [D]: the piece constructor (1483587c0) takes y0 from the
  dimension's +0xcc (sea level, 63) and y1 from +0xca (its top); sapling's `y0: 63` over the union.

## The lists and caps [D]

141f82140 registers each list; 143906b20 unpacks each packed entry into a `MobSpawnRules` (+0xb8 u16
least, +0xba most; +0xbc/+0xbd/+0xbe == 2 surface/underground/water spawner; +0xc0/+0xc2 own caps if
+0xc4; +0xc6/+0xc8 brightness and +0xca weather adjust if +0xcc). It sets `mPersistence` (+0x184) 0
for every entry: no structure mob is persistent, as sapling's `Natural`.

| structure | entry (rules weight, unread by the pass; herd) | spawner | own caps | brightness |
| --- | --- | --- | --- | --- |
| fortress | zombie_pigman 5, magma_cube 3, blaze 10, skeleton 2, wither_skeleton 8 (2-3) | underground | - | - |
| monument | guardian 1 (2-4) | water, underground | 0 / 40 | - |
| swamp_hut | witch 1 (1) | surface | - | 0-8, weather |
| outpost | pillager 1 (1-2); pillager `spawn_as_illager_captain` 1 (1) | surface, underground | 7 / 7; 1 / 1 | 0-8 |

`canSpawnInConditions` (1439077f0) gets the attempt column's top spot (its water and lava) and
brightness 0, so the entries' brightness asks nothing but in a thunderstorm (darken 10, weather
entries). The light gates are the kinds' own, in `spawnMob` (14118caf0) by `checkSpawnRules`:
`Monster::checkSpawnRules` (14298fc00) is `isDarkEnoughToSpawn` (raw brightness under 8, thunder
dimmed by 10; overworld block light 0 too, fcn.1415f7530) and `Mob::checkSpawnRules` (box clear,
the block below, a walk target of 0.5 less the brightness at least 0: raw 12 at most). Pillager and
blaze override the dark ask to true; the magma cube's rules are trivial; the zombie pigman's and the
guardian's own are not decompiled [U]. So the hut witch wants raw brightness under 8 (sapling
`Checks::Dark`), skeletons and wither skeletons too, pillagers and blazes 12 at most (`Walks`).

`isSpawnPositionOk` (14118c440): the nearest player at least 24 and at most min(128, 44 at a chunk
tick range of 4) off, the spot water iff the entry spawns in water (lava alike), its cell and the one
over not solid. Then the caps, before each member: the entry's own caps clip the herd by the kind's
count on its side (surface or underground, by the entry's surface flag), the 200 mobs clip it, and
each member wants its category at most its cap on that side. The category is the kind's spawn rule
file's `population_control` (141d3f2d0): pillagers count as `pillager`, caps 8 / 8, not `monster`.
Mobs count by the surface flag they spawned with (`_updateBaseTypeCount` 1415ee820, actor vt+0x180),
not where they stand (sapling `herds::Surfaced`). Sapling: `spawning/structures/lists.rs`, tests
`fortress_entries_are_picked_alike`, `witches_want_the_dark_and_pillagers_only_the_shade`,
`an_outpost_keeps_to_its_pillager_caps`.

Drilled [L] (`t3spawns`, twelfth3_drill.rs, research/captures/bds-t3spawns.txt; BDS on a default seed 20261003
world): off the outpost at noon, pillagers came at (1888.0, 130, -1128.0) and twice at (1868.0, 131,
-1116.0), the middles of the tower's strip in chunk (118, -71) and of the feature piece in chunk
(116, -70) found down to their floors, on whole x and z: no inset, no darkness asked, a herd of two at
once. Of 11 seen, 6 were left after 150 s (natural, not persistent). Off the hut, 150 s at noon and
150 s at midnight brought no witch besides the one generation placed (and a natural swamp witch at
x .49); `t3deep` 31 blocks off at noon brought none either, so sapling's hut witch takes spots of
brightness 7 at most (as the natural witch rule; the exact gate is [U]). Over the monument at noon,
guardians came at y 60.99999 on the building's per-chunk middles (x -387, -376, -360, -344, -334;
z -371, -360, -344, -328, -318), y the middle's own (61): the search starts at the middle, not a
cell above it. In the fortress, zombie pigmen, wither skeletons (three at once), blazes (two),
magma cubes and skeletons came on whole x and z: every entry of the list.

Sapling's `t3deep` (drill_server `SAPLING_DRILL_WORLD=generated SAPLING_DRILL_SEED=20261003`, debug,
lagged): no hut witch at noon, as BDS; guardians on the same five x and z at y 61, 26 left after
150 s (BDS 15); every fortress entry on whole x and z. Left: sapling's fortress pass reached x 178
to 283 and most of its ~500 mobs despawned at once past the 44-block cap, where BDS showed ~45 mobs
between x 231 and 277; whether BDS spawns and despawns them unseen is not known. A first sapling run
was on peaceful (the drill's first lines came before the world answered) and showed the pass
spawning monsters there, now refused. Sapling's `t3spawns`: pillagers on the same middles as BDS
(1868, 131, -1116; 1880, 142, -1128; ...) and witches at the hut's middle (3843, 72, -1820) only at
midnight, each gone at once past 44 blocks (BDS sent none: unseen or not spawned). Left: 26
pillagers after 150 s against BDS's 6, sapling's caps filling with pillagers alone where BDS's held
natural monsters too. Captures research/captures/{bds,sapling}-t3{spawns,deep}.txt (the actor's
AddEntity, RemoveEntity and answers kept).

### The pass, drilled again [L]

`t9spawns` (twelfth9_drill.rs; two runs a side, research/captures/{bds,sapling}-t9spawns{,-run2}.txt,
seed 20261003 as above): 150 s 30 to 40 off the outpost at noon, 31 off the hut at midnight, over the
fortress; the answers of `testfor` at the end of each:

| | pillager | witch | blaze | wither skel. | skeleton | magma cube | zombie pigman |
| --- | --- | --- | --- | --- | --- | --- | --- |
| BDS | 10, 12 | 5, 2 | 0, 2 | 2, 0 | 5, 3 | 1, 3 | 5, 8 |
| sapling | 5, 2 | 2, 1 | 1, 4 | 0, 1 | 1, 0 | 3, 0 | 12, 8 |

The 26 against 6 is gone: sapling's pass now has the 24 to 44 player distance, the pillager category
(8 / 8) and the entries' own caps, and counts mobs by their spawn flag. BDS's outpost counts hold 8
pillagers its template placed at first sight (x .3 and .7 among whole cells), which sapling does not
place (sapling-worldgen places no template actors); the pass spawned 18 and 18 on BDS, 12 and 7 on
sapling, on the same middles. The fortress pass's whole cells now keep to x 232 to 266 (BDS 232 to
277), where they reached 178 to 283. Left: sapling's fortress pass spawns fewer (14 and 10 mobs
against BDS's 33 and 29) and keeps fewer (2 and 6 against 8 and 9); sapling's natural zombie pigmen
come three to a herd where BDS's came alone. Neither is decompiled.

Again after the twelfth wave's group 12 (template actors, the pigman's and Monster's own checks,
pillagers then still under the block-light test:
mob-spawning.md "Own spawn checks"; research/captures/sapling-t9spawns-t12{,run2}.txt against the
BDS pair above). First shown in the Nether box, blazes, wither skeletons and skeletons: BDS 26 and
18, sapling 22 and 10 (were 7 and 9); kept at the end BDS 7 and 5, sapling 0 and 6. Zombie pigmen:
BDS 17 and 17, every one alone on its cell; sapling 27 and 8, every one alone (were 27 and 23 in
herds of 2 and 3). The outpost's testfor: BDS 10 and 12, sapling 15 and 9, the 8 its template
placed among them, at the cells BDS showed. Left: BDS's skeletons (5 and 11 shown), none in
sapling's runs [U]. Rechecked 2026-11-05: 141f82140 registers the skeleton (0x110b22) at weight 2,
herd 0x30002 (2 to 3), and the wither skeleton (0x110b30) at 8, as `lists.rs`; BDS's skeletons stood
on whole cells, so the structure pass made them (16 against 12 wither skeletons over the two runs),
which weight 2 of 28 does not explain. Settled [D] (thirteenth3j): the list item's own weight
(+0 of each 0xd8 entry, rbp+0x520 to +0x880 in 141f82140) is 1 for all five, and the pass's pick
(1415eebf0: `nextInt(total)`, then the +0s walked) reads only that; the 5, 3, 10, 2, 8 sit in the
unpacked rules (+8) and go unread. So each kind is one in five; BDS's structure-pass kinds over both
runs (whole cells): blaze 16, zombie pigman 9, magma cube 9, skeleton 16, wither skeleton 12.
Sapling's `pick` is now even. Re-drilled (research/captures/sapling-t9spawns-j{1,2}.txt; unique ids
on whole cells over the fortress, blaze / magma cube / skeleton / wither skeleton / zombie pigman):
BDS 8/3/11/7/4 and 8/6/5/5/5 (33, 29), sapling 8/6/3/4/6 and 9/9/10/3/4 (27, 35).

Sapling ports the pass with natural spawning's dice; test
`witches_appear_in_a_hut_piece_where_nothing_else_does`. The entries' conditions, caps and
persistence: "The lists and caps" above.

## Template actors

Decompiled [D]: each template piece runs `postProcess` (vtable slot 4), which places the template
and hands each data marker in the chunk's box to the piece's handler, then `postProcessMobsAt`
(slot 5), which spawns what the handler filed if it is inside the box. Every mob comes persistent
(`PersistentComponent`), at the corner of its marker's cell (cvtsi2ss x, y, z) unless said
otherwise. The marker cell is the template block's own cell (`Rotation::apply`), as the blocks.

| structure | handler / mobs | markers |
| --- | --- | --- |
| outpost | 148359310 / 1483599e0 | `cage` iron golem (x + 1 in a piece turned once), `pillager`, `captain` (event `spawn_as_illager_captain`), `Group of Allays` nextInt(3) + 1 allays; each cell set to air; `topChest` seeds the chest below |
| mansion | 1480b2360 / 1480b2a00 | `Mage` evoker, `Warrior` vindicator (cells left as they are), `Group of Allays` as the outpost's; spawned evokers, vindicators, allays |
| swamp hut | 14836de70 / 14836f380 | no marker: once per piece (flag +0xe8), a `witch` then a `cat` (event `spawn_midnight_cat`, shown `Variant` 9) at `getWorldX/Z(2, 5)` + 0.5, box y0 + 2, whatever the box; both persistent |
| igloo | 14836fd90 / 14836f7f0 | `Villager`, `Zombie Villager` (v1 actor types, shown as their successors) at the cell's middle, villager first; basement only |
| end city | 14831aaa0 / 14831a670 | `Sentry` shulker; `Elytra` is an item frame block, not an actor |
| pool elements | 145e1e2a0 / 141ef28c0 | the metadata `namespace:id[:event]`; abandoned villages' rules (1480bbe90) make `villager_v2:spawn_from_village` a `zombie_villager_v2:from_village` |

The `Group of Allays` marker (its `nextInt(3)` draw too) is gated on the world's base game version
being at least 1.19.0, the constant 14cc486c0 (static init 14105b908: `0x1000000130001`; the
1.18.0 once read for it is its neighbour 14cc486a0, 14105b872). The mansion (1480b286a) skips it
when `140247750` (SemVersion less-than) says the version is under the constant; the outpost
(1483596bc) spawns when `14087de00(constant, version)`, which is less-or-equal, holds. A version
with its "any" flag, or a constant not yet filled, spawns them. Current worlds are newer, so both
spawn allays and draw, as sapling's do (checked 2026-11-05, thirteenth3s).

Ocean ruins and shipwrecks spawn nothing (slot 5 is a no-op). Java templates' `entities` lists are
read but never placed (141eedcf0 parses them, `placeInWorld` 141eeb8d0 skips them): the plains
villager templates and camp cushions show nothing. A bastion's .mcstructure elements place their
saved entities in a second pass (145e1c910, `StructureTemplate::placeInWorld` 141f52300 with
blocks ignored), full NBT through `ActorFactory` 141ce8200, no event and no added persistence.

Drilled [L] (`t12actors`, `t12actors2`, twelfth12_drill.rs, seed 20261003 default world;
research/captures/bds-t12actors{,2}.txt; the BDS world dumped with `actor_dump --all`): with
`doMobSpawning` off BDS placed none of them; with it on, by the outpost at (1896, -1112) the 7
pillagers and the captain on the tower and the cage's golem; in the village at (1752, -1016) 6
villagers, 3 cats and a golem; in the mansion at (-8184, 15944) 3 evokers and 9 vindicators in
view; in the bastion at (-328, 248) 22 piglins, 13 brutes and 14 hoglins. All Persistent 1,
NaturalSpawn 0 (the evokers too, though 1480b2a00 drops its add result).

Sapling: sapling-worldgen files them as `TemplateActor`s on the chunk (`ChunkShape::actors`,
`Canvas::add_actor`); the shard takes them on arrival into a per-layer wait list
(actors/templates.rs) and makes them as BDS's mob step does (below), `SpawnReason::Structure`, no
spawn sound; after that they are kept as any mob and never made again. With `doMobSpawning` off
they are dropped. Tests
`outpost_tower_places_its_pillagers_where_bds_did`,
`village_markers_place_villagers_cats_and_a_golem_where_bds_did`,
`mansion_markers_place_its_illagers_where_bds_did` (structure/actor_tests.rs: every BDS cell
matched), `a_template_mob_comes_once_with_its_column_and_stays_with_it`,
`a_template_mob_waits_until_its_column_ticks_with_its_neighbours_loaded`. Live, sapling's
`t12actors2` (research/captures/sapling-t12actors2.txt, the drill since waiting on a barrier pad
and standing longer, as sapling generates slower) showed the outpost's 8 and its golem, the
village's 6 villagers, 3 cats and golem, each at BDS's first-sight cell, and the mansion's 2
evokers and 6 vindicators it had loaded, at BDS's cells too. Left: the bastion's
entities (they need the templates' entity NBT in templates.bin and the transform of 141ce8200
fitted against the dump); five mansion mobs sapling places in chunk x -511 that BDS showed none of
(explained below, "When they come") (2026-11-05, `t12actors2` rerun, research/captures/bds-t12actors2-run2.txt, and its world dumped: chunks -511 and -510 saved, natural
mobs at x -8170 and -8158, no illager east of x -8189; the five are evokers at (-8174, 139, 15899)
and (-8171, 130, 15899), vindicators at (-8172, 129, 15906), (-8171, 139, 15900) and
(-8170, 129, 15906). 1480b2a00 keeps each list on the piece and spawns only what lies in the box it
is handed, keeping the rest; the caller's box was not read); the igloo and end city (none located
on the seed) are undrilled.

When they come [D, L] (Linux gdb, scripts/bds_gdb.py; the chunk tick 0xc9e2940): a chunk's
FinalizedState (+0x1114) starts at 0; while 0 the chunk asks `hasChunksAt` for its box grown by one
block, drains its pending ticks (100 a tick, giving up after 200 ticks, +0x1110) and becomes 1. The
next tick with 1 runs the mob step (0xc9e3156): biome mobs, then the generator's
`postProcessMobsAt` (0xc973030 per structure, 0xd251430 per piece, gated on `doMobSpawning`) with
the box x0..x0+16, z0..z0+16. Only ticking chunks get there. So a column's template mobs come the
tick after it first ticks with its eight neighbours loaded (the neighbours need not tick); pieces
stay in memory, so mobs waiting in an unloaded column keep waiting, and none survive a restart.
The five chunk -511 mobs [L]: `jxmansion` (thirteenth3j_drill.rs) with
`client-side-chunk-generation-enabled=true` showed none (research/captures/bds-jxmansion.txt):
client generation leaves the server's neighbours of chunks (-511, 993/994) unloaded. With it false
BDS made all five at sapling's cells (research/captures/bds-jxmansion-servergen.txt): 5 evokers,
12 vindicators by the end, all shown 12 s in. Sapling's `jxmansion` (research/captures/sapling-jxmansion.txt,
the testbox loaded, the shard behind its tick, the pad's fill out of the world) showed its first at
62 s and 2 and 8 by the end, each at a BDS cell: its generation, not the rule, kept the rest back.

Abandoned villages' golems [D]: 1480bbe90 registers `minecraft:abandoned_golems` with the same
actor rule processor (vtable 14a89ad80) as `abandoned_mobs`, matching
`minecraft:iron_golem:minecraft:from_village` with an empty result: the golem is dropped, as
sapling does.

The bastion's saved actors [L] (`az3bastion`, thirteenth3az_drill.rs: both bots straight to
(-328, 248) in the Nether a minute; research/captures/bds-az3bastion.txt, its world dumped with
`actor_dump --all`): BDS showed 24 piglins, 13 brutes and 14 hoglins (51; the 22 piglins counted
above were a miscount: bds-t12actors2.txt shows the same 51 at the same places). Each is a
`bastion/mobs/*.mcstructure` piece's one saved actor (crossbow_piglin, sword_piglin, melee_piglin,
melee_piglin_always, hoglin), made from its compound: kind, `definitions` as saved (`+ranged_unit`
or `+melee_unit`, `+go_back_to_spawn` ...), its `Mainhand` (crossbow, golden sword, golden axe),
Persistent 1, NaturalSpawn 0, no spawn event. Its feet are the saved `Pos` less the template's
`structure_world_origin`, turned with the piece about the middle of its block cell (a quarter turn
takes fraction (x, z) to (1 - z, x)); its yaw is the saved yaw plus 90 a quarter, wrapped to
-180..180 (BDS showed -180 for 0 turned twice); its `HomePos` (brutes) turned as the feet (the
dump: a turned brute at x -342.628 saved home x -342, an unturned one its own cell). Gated as
the other template mobs (none with `doMobSpawning` off, bds-t12actors.txt). BDS places feet and
`HomePos` alike one block below the template's offset (the dump's homes, not a fall): on the
parent's floor, under the piece's jigsaw cell, which ends as air (final state `blackstone_bricks`).

The piece's turn: the hoglin stables' mob jigsaws are `aligned`, so the mob piece takes the turn
whose jigsaw top matches. Sapling first read the mcstructure jigsaw's `rotation` state as
north, east, south, west; it is Bedrock's south, west, north, east (scripts/mcstructure.py
`TOPS`), and with the old reading every mob under an aligned jigsaw (29 of 51) stood a half turn
from BDS's while the blocks (one cell, no block of its own) replayed the same. Fixing it moves
only the five mob templates' jigsaw state and orientation bytes in templates.bin; the palette is
unchanged.

Sapling: scripts/templates_to_rust.py appends the mcstructure actors to templates.bin (`saved_actors`,
after the state table, so the earlier bytes are unchanged); template/load.rs reads them into
`Template::actors`; a pool element files them with its blocks (jigsaw/actors.rs `add_saved`) as
`TemplateActor`s carrying a `SavedCompound`; the shard's mob step brings each in from its compound
as `/structure load` brings a structure's entities (actors.rs `bring_in_saved`), the home rewritten.
Tests `bastion_mob_pieces_place_their_saved_actors_where_bds_did` (all 51 feet within 0.001 and
yaws within a byte step of BDS's, the brutes' homes as the dump),
`bastion_mob_pieces_keep_their_saved_actors`, and the shard's
`a_bastion_piece_s_piglin_comes_as_its_template_saved_it`. A templates.bin made before this has
no actor section and loads with none: regenerate it. Sapling's drill
(research/captures/sapling-az3bastion.txt, generated world, seed 20261003) shows all 51 at BDS's
positions (within 0.01), yaws and mainhands; its 7 more (4 piglins at (-333.51, 61, 234.49), 3
zombie pigmen) are natural packs at cell middles, which BDS's minute happened not to spawn.

The swamp hut's witch and black cat (row above; research/captures/bds-h3keep.txt, both at
(3842.5, 72, -1818.5) at first sight, the dump Persistent 1, NaturalSpawn 0): sapling-worldgen's
hut.rs files both, the cat's event setting `Variant` 9 (creatures/looks.rs); test
`swamp_hut_places_its_witch_and_black_cat_where_bds_did`.

## Patrols

See patrols.md.
