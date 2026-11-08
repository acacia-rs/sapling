# Patrols

Moved out of structure-spawns.md (its "Patrols" section).

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
existing record). The record's first value [D]: the `ActorSpawnRuleGroup` constructor (141d3b7f0)
files each delay id with its least delay (rule +0xa0) as the end, an absolute world age in seconds,
so the first wait ends at 600 s, which the 6000 s age gate already covers. No save path was found
(no key, no loader filling the map): every load starts the waits again [U for an unlabelled one].

The village filter (lambda 14391c620) [D]: for each player whose distance to the spot is within
`distance`, `VillageManager` slot 1 (142a3c120) asks each village's bounds (Village +0x1a0 to
+0x1b4) grown by the tolerance (32) on all six sides, y too; one holding the player refuses. A
village's bounds are its first claimed point's box, ±32 across and ±12 up (142a28650), widened by
the recompute (142a22280) to take in each later claimed point (the point alone).

`mobevents` [D]: 14085b130 writes the leveldb key `mobevents`, a compound of bytes
`minecraft:pillager_patrols_event`, `minecraft:wandering_trader_event`,
`minecraft:ender_dragon_event` and `events_enabled` (names from 140859ed0). A BDS world saved after
`t12actors2` (2026-11-05) held all four, 1 each, sorted by name.

Sapling: spawn-rules.py writes `event=`, `age=`, `delay=least~most:chance:id`, `village=` fields
(`sapling_data::Gates`, spawn_rules/gates.rs; `kind<event>` permutes split by `kind_and_event`);
sapling-shard spawning/delays.rs asks them in the cluster; the world's age is `Level::age`
(level.dat `currentTick`, advanced every tick). The herd's first member gets
`promote_to_patrol_captain`, the rest `spawn_as_patrol_follower`. The `/mobevent` switches are
`sapling_api::MobEvents` in plugin state, which the cluster reads (an event runs while it and
`events_enabled` are on); the first wait ends at age 600 s and waits are not saved, as BDS; the
village filter is raids/village.rs `within_bounds`: the bells and beds within 48 of a held
village's middle, their box grown by ±32 / ±12 / ±32 and the tolerance [U: BDS's claimed points,
the first one's box]. The switches are saved under `mobevents`, all four (sapling-leveldb
mob_events.rs; `Level::mob_events`), and the first world added that kept some sets them
(sapling-shard spawning/switches.rs). Tests `a_patrols_gates_read`,
`a_folder_brings_its_time_and_mobs_and_takes_the_time_back`, `a_patrol_waits_for_the_worlds_age_then_its_delay_and_its_chance`,
`a_mob_event_switched_off_spawns_nothing`.

Movement (pillager `patrol_captain` / `patrol_follower` groups) [P]: the captain
`move_to_random_block` (block_distance 512, within_radius 8, speed 0.55), its `hold_ground`
(prio 5, min_radius 10) acting on a target only; a follower `follow_target_captain`
(follow_distance 5, within_radius 64, speed 0.8).

`MoveToRandomBlockGoal` [D] (vtable 14a9301b0, built 148881870; speed +0x58, within_radius² +0x5c,
block_distance +0x60): canUse (148838db0) asks RandomPos (14597a590, 14597ab80) for ten tries of x
and z `nextInt(2r + 1) - r`, y `nextInt(17) - 8`, the best walk value winning, a uniform square of
±512; the target's y becomes its column's height, and a target whose block above or itself fails
a block check is dropped. Its tick (1488393e0) waits while a path runs and ends within the radius;
farther than an attribute's value (0x14cd00528 +0x7c, taken as follow range 64, compared squared
[U]) it heads for its position plus the unit direction times half that value, in whole blocks; a
move of under a block twice running steps 10 along the main axis, `nextInt(2)` turning it; after 4
stalls it rests 400 ticks. Navigation walks each leg, hence the compass and diagonal lines.
`TargetCaptainID` is kept (`Mob::readAdditionalSaveData` 1429b4e60); the follow goal's leader is
an `is_illager_captain` (lambda 140e292a0).

Sapling: creatures/raids/patrols.rs, the block a uniform square of ±512, 32-block legs ending on a
whole cell, each walked cornerwise until level on one axis, then straight; the group kept in the
stored `definitions` (`+patrol_captain`, `+patrol_follower`). A follower takes the nearest captain
within 64 each tick, not a kept id. Left: the stall sidestep and rest; the target's height and
block checks. Tests `a_patrol_captain_wanders_and_its_followers_keep_up`,
`a_kept_patrol_captain_walks_on_and_keeps_its_group`, `a_captain_walks_a_leg_cornerwise_then_straight`.

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
Tried once (`tt1patrol`, bds-tt1patrol.txt, fourteenth1t): the flat drill world's level.dat
`currentTick` set to 140000 (7000 s) and restored after, `KEEP_WORLD=1`, both players creative at the
spawn at midnight on easy, `/mobevent` patrols and events on, 480 s: no pillager came (32 natural
monsters did: zombies, creepers, skeletons, spiders, two zombie horses). One run has about one 21 in
100 roll, as the first wait is already over at that age. To see patrols, raise the rule's
`spawn_chance` in the run's pack (memory bds-pack-override-probe) or run several such nights.
