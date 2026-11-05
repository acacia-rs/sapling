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

Sapling files these in the column's own post-process (`spawn_areas::file`) and saves them with
generated columns (`StoredColumn::spawn_areas`); test `spawn_area_tests.rs` against the records.
Left: whole-only columns of an outpost (a column its box meets and no piece does) have no
post-process in sapling, so no record; only whole boxes are there, which these four structures do not
spawn from. Outpost pieces past the start chunk find their height by the run's post-process order, as
their blocks do. Data-driven structures (trial chambers, abandoned camps, ...) are not filed.

## The spawner [D]

`BedrockSpawner::tick` (1415eebf0), after the attempt's column:

- Natural clusters: at each level whose floor block lies in a piece of a structure with a list
  (1438cfcd0), the cluster is skipped. All four register for pieces only (filter flag 1, 141f82140).
- The pass: the ticked chunk's pieces of such structures give their middles (1438db9d0): x0 plus
  half the width, rounded down, z alike, y the piece's top plus the y offset. `max(2, n) / 2` tries,
  each `nextInt(n)` a middle, `findNextSpawnBlockUnder` down from it (water only when the column's
  first spot is under water), the pieces at the found floor asked again (1438cf5c0, then 1438cfcd0),
  an entry by weight (all 1), `canSpawnInConditions`, a herd of `min + roundf(f² (max - min))`, then
  `isSpawnPositionOk`, the 200-mob and category caps as natural spawning, the mobs at the floor's
  corner plus one up (`1415efb20`: integer x and z, no inset), persistent only where the entry says.
  A refused spot ends the pass.
- Lists (all monster): fortress zombie_pigman, magma_cube, blaze, skeleton, wither_skeleton (2-3
  each); monument guardian (2-4); swamp hut witch (1); outpost pillager (1-2) and a pillager by
  `spawn_as_illager_captain` (1).

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

Sapling ports the pass with natural spawning's dice; test
`witches_appear_in_a_hut_piece_where_nothing_else_does`. [U]: the entries' own conditions (light for
the hut, fortress and monument; the outpost's and monument's extra fields), the context position `canSpawnInConditions` gets, and
which entries are persistent; sapling asks only the medium (water for guardians, else air on a full
block) and leaves every one natural.

## Patrols [P, not built]

`pillager_patrol` (vanilla 1.14, 1.15, 1.18.0 packs): `mob_event_filter`
`minecraft:pillager_patrols_event`, `world_age_filter` min 6000, `delay_filter` 600 to 660 with
`spawn_chance` 20 (one identifier per difficulty), `player_in_village_filter` distance 48 tolerance
32, `spawns_on_surface`, distance 24 to 48, brightness 0 to 7 (not adjusted for weather), not in
mooshroom, nether or end biomes; herd 2 to 5 of `pillager<minecraft:spawn_as_patrol_follower>`, the
first by `promote_to_patrol_captain`. Not built: spawn-rules.py leaves the rule out (no delay, world age,
mob event or village filters in sapling-data), and patrol movement is not ported.
