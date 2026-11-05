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
No block changes: `tools/replay-all.sh` 0 wrong, 0 missing (2026-11-05).
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
  `isSpawnPositionOk`, the caps ("The lists and caps"), the mobs at the floor's
  corner plus one up (`1415efb20`: integer x and z, no inset). A refused spot ends the pass.
- The outpost's whole box y 63 to 319 [D]: the piece constructor (1483587c0) takes y0 from the
  dimension's +0xcc (sea level, 63) and y1 from +0xca (its top); sapling's `y0: 63` over the union.

## The lists and caps [D]

141f82140 registers each list; 143906b20 unpacks each packed entry into a `MobSpawnRules` (+0xb8 u16
least, +0xba most; +0xbc/+0xbd/+0xbe == 2 surface/underground/water spawner; +0xc0/+0xc2 own caps if
+0xc4; +0xc6/+0xc8 brightness and +0xca weather adjust if +0xcc). It sets `mPersistence` (+0x184) 0
for every entry: no structure mob is persistent, as sapling's `Natural`.

| structure | entry (weight, herd) | spawner | own caps | brightness |
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
`fortress_entries_go_by_their_weights`, `witches_want_the_dark_and_pillagers_only_the_shade`,
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

Sapling ports the pass with natural spawning's dice; test
`witches_appear_in_a_hut_piece_where_nothing_else_does`. The entries' conditions, caps and
persistence: "The lists and caps" above.

## Patrols

`pillager_patrol` (vanilla 1.14, 1.15, 1.18.0 packs) [P]: `mob_event_filter`
`minecraft:pillager_patrols_event`, `world_age_filter` min 6000, `delay_filter` 600 to 660 with
`spawn_chance` 20 (one identifier per difficulty), `player_in_village_filter` distance 48 tolerance
32, `spawns_on_surface`, distance 24 to 48, brightness 0 to 7 (not adjusted for weather), not in
mooshroom, nether or end biomes; herd 2 to 5 of `pillager<minecraft:spawn_as_patrol_follower>`, the
first by `promote_to_patrol_captain`; `population_control` pillager.

`_spawnMobCluster` (1415ed230) [D]: the delay's end (`getDelayEnd` 141d3f460, `mSpawnDelayStartMap`)
goes into the conditions; `canSpawnInConditions` refuses unless the world's age in seconds (ticks /
20) is past it, and below the world age minimum (seconds). Once the herd's spot passed: with a delay
and a chance under 100, `nextInt(100) > chance` refuses and restarts the wait (so 21 in 100 pass);
anything spawned restarts it too (141d3f5e0: now + least + `nextInt(most - least)`, only for an
existing record). The village filter (lambda 14391c620) asks, for each player within `distance` of
the spot, the village manager near the player with the tolerance; one found refuses. The record's
first value is not known [U]: sapling starts a wait the first time a rule asks.

Sapling: spawn-rules.py writes `event=`, `age=`, `delay=least~most:chance:id`, `village=` fields
(`sapling_data::Gates`, spawn_rules/gates.rs; `kind<event>` permutes split by `kind_and_event`);
sapling-shard spawning/delays.rs asks them in the cluster; the world's age is `Level::age`
(level.dat `currentTick`, advanced every tick). The herd's first member gets
`promote_to_patrol_captain`, the rest `spawn_as_patrol_follower`. Left: every `/mobevent` is taken
as on (the switches live in the sapling crate); waits are not saved; a village is sapling's bell or
bed within 48 of the player (raids/village.rs), without BDS's bounds and tolerance. Tests
`a_patrols_gates_read`, `a_patrol_waits_for_the_worlds_age_then_its_delay_and_its_chance`.

Movement (pillager `patrol_captain` / `patrol_follower` groups) [P]: the captain
`move_to_random_block` (block_distance 512, within_radius 8, speed 0.55), its `hold_ground`
(prio 5, min_radius 10) acting on a target only; a follower `follow_target_captain`
(follow_distance 5, within_radius 64, speed 0.8). Sapling: creatures/raids/patrols.rs, the
captain's block a uniform square of ±512 (MoveToRandomBlockGoal not decompiled [U]); test
`a_patrol_captain_wanders_and_its_followers_keep_up`. Patrol groups are not saved.

Drilled [L] (`t9patrol`, `t9patrol2`, `t9patrol3` on the flat world, sapling
`SAPLING_DRILL_WORLD=open`; research/captures/{bds,sapling}-t9patrol*.txt): a summoned captain and two
followers 6 off it, the players 40 to 60 away in creative. BDS's captain walks about 1.6 blocks a
second, as sapling's, along compass and diagonal lines turning now and then (west from (60, 0),
south-west from (40, 30), south-east then south from (-60, 0)): a path to a far block, not toward
the players. Sapling's walks a straight line. BDS's first follower kept 1.3 to 5.6 off the captain;
the second lagged to 15 to 22 off before closing in again in two of three runs [U]; sapling's kept
3.4 to 8. Sapling's captain stopped once it left the players' tick region (64 off), as any mob there.

Undrillable naturally: a patrol wants a world 6000 s (100 minutes) old, then a 600-660 s wait, then
a 21 in 100 roll that restarts the wait on a miss; a drill world is fresh on both sides.
