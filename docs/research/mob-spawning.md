# Natural mob spawning and despawning (BDS)

What sapling copies of BDS's natural spawner (`crates/sapling-shard/src/spawning*`), from where, and
what is left. [P] = read from the vanilla behaviour packs, [D] = decompiled (bdsre), [U] = unverified
(believed BDS behaviour, no evidence here).

## Data [P]

`crates/sapling-data/data/generate/spawn-rules.py` turns every `spawn_rules/*.json` of the vanilla packs
(`spawn_rules.brarchive`, newest pack winning) into `spawn-rules.tsv`: 97 conditions of 55 kinds, and each
entity's `minecraft:despawn` into `despawns.tsv` (65 kinds at 32/128, the fish and nautili at 32/40).
Format: sapling-data README "Spawn rules". Biome tags come from `BiomeDefinitionList.bin` (BDS sends
every vanilla biome with id `0xffff`; the stored ids are the table in `spawn_rules/biomes.rs`).

Rule components and how they are read:

- `spawns_on_surface` / `spawns_underground`: the spot is above / below the highest block with collision
  in its column. The nether's roof makes every nether spot underground; the end has none. [U]
- `spawns_underwater`: the spot's cell is water; a rule without it never spawns in water, one with it
  only there. `spawns_lava`: air over lava (striders). [U]
- `brightness_filter`: `World::brightness_at` (BDS's `getRawBrightness`) with the sky dimmed by the time;
  with `adjust_for_weather`, by rain and thunder too. [U for the weather split]
- `difficulty_filter`, `height_filter`, `distance_filter` (to the nearest player), `spawns_on_block_filter`,
  `spawns_on_block_prevented_filter` (the block under the feet), `weight`, `herd` (size only),
  `permute_type`, `density_limit` (mobs of the kind near the player, surface or underground).
- Biome filters: a list is `all_of`; `has_biome_tag` with `==`/`!=`/`not`; `is_snow_covered` is the
  biome's minimum snow level above 0 ("Snow cover" below).

`pillager_patrol`'s world age, delay, mob event and village filters are `sapling_data::Gates`
(structure-spawns.md "Patrols"). Left out: the guardian (ocean
monuments: structure spawns, docs/research/structure-spawns.md); the slime and the phantom have their code gates (below); herd events
are husbandry.md "Herds"; `disallow_spawns_in_bubble` (the axolotl's) refuses a bubble column cell
(spawn-rules.tsv `no_bubble`, `spawning/spots.rs` `in_bubble`; [P], BDS's own test not read); blazes,
wither skeletons, cats and pillagers have no natural conditions (structures spawn them).

## Engine [D]

Decompiled from the bds-blocks build (`BDS_EXE=acacia/.testserver/bds-blocks/bedrock_server.exe`, cache
`6ab54bee`). BDS runs the legacy `BedrockSpawner` (vtable `14a753720`).

| VA | What |
|---|---|
| 1415eebf0 | `BedrockSpawner::tick(region, volumeData, chunkPos)`: one ticked chunk |
| 1415ed230 | `_spawnMobCluster`: rule pick, caps, herd |
| 1415ee3d0 | `_spawnMobInCluster`: one member |
| 1415ee820 | `_updateBaseTypeCount`: the counts the caps read |
| 14118c0f0 | `findNextSpawnBlockUnder` |
| 14118c440 | `isSpawnPositionOk` |
| 1439077f0 | `canSpawnInConditions` (brightness, height, difficulty, ...) |
| 141d3f910 | spawn rule parser (distance defaults) |
| 141ba30e0 | `DespawnComponent::_canDespawn`; 141ba3960 its soft rules |
| 14165cf60 | `BaseMobSpawner` ctor |

- Gates: `doMobSpawning` (game rule index 7) must be on; then `nextInt(2000) <= 10`: an attempt in
  11 of 2000 ticks of each ticked chunk. sapling ticks the chunks within 4 (BDS's default
  `tick-distance`, a circle here [U]) of each player.
- Column: a random one of the chunk. From the top down `findNextSpawnBlockUnder` finds the first spot
  (`isOnSurface`), then a cluster runs at every lower spawnable level of the column. sapling: a level is
  the top cell of a run of water, a cell over lava, or an air cell over a full block; "surface" is
  above the column's highest block with collision [U: BDS's own surface flag for the nether].
- Distance: the nearest player (3D) must be within the rule's `distance_filter`, default min 24, max
  128; at tick range 4 the max is clamped to 44.
- Brightness: `getRawBrightness` with the dimension's sky darkening (time and weather). With
  `adjust_for_weather`, when rain times thunder is over 0.9, it is recomputed with sky darkening 10.
- Herd: one of the rule's herds by `nextInt(count)` (default 1 to 1), size
  `min + roundf(f² (max - min))` with `f` a `nextFloat` (`_spawnMobCluster` 1415ed230, `roundf` import
  14a5bd080), so small herds are likelier; every member on the same cell, at (x + 0.49, y, z + 0.49) in
  worlds of 1.21 on. Herd events: husbandry.md "Herds"; guaranteed members are not built.
- Caps: counts over the 9×9 chunks round the cluster's chunk, split surface/underground: per category
  and per kind (`density_limit`, clipping the herd to what is left). The category cap is asked before
  each member, `0 < cap && count < cap`, so a category ends at it (the structure pass's own reading
  is one over: "Own spawn checks" end). Every spawn is also clipped so the world
  holds at most 200 spawnable mobs (BDS: those ticked last tick). The caps are floats the base
  `Dimension` constructor stores (141878200, stores 141879818 to 141879884), surface at +0x104 and
  underground at +0x120, indexed by `ActorSpawnRuleGroup`'s category (141d3b7f0): animal 4/4, monster
  8/16, water_animal 36/36, villager 0/0, ambient 0/2, cat 4/0, pillager 8/8, the same in every
  dimension. Sapling (`spawning/herds.rs`) uses these for its four categories; test
  `herd_sizes_lean_small_as_bds_squares_the_roll`.
- Monster spawner blocks: built, docs/research/spawners.md.

## Slimes [D]

`ActorSpawnRuleGroup`'s lambda 28 gives `minecraft:slime` alone a placement check (`1429983d0`, which
bdsre's labels call "SlimeBlock factory"), on top of its pack rule (surface or underground, any
monster, swamp, mangrove or frozen biome). sapling: `spawning/gates.rs`, tested as a rule is admitted.

- Below y 39: only in a slime chunk. An MT19937 seeded `(cx * 0x1f1f1f1f) ^ cz` (block coordinates
  `>> 4`, no world seed), first draw `% 10 == 0` (`imul 0xcccccccd; ror 1; cmp 0x1999999a`).
- y 50 to 68: raw brightness under 8 and not over a `nextInt(8)` of the level's random, then a
  `nextFloat` under the moon's fullness, `[1, .75, .5, .25, 0, .25, .5, .75]` (`14a75c000`) by
  `|time / 24000| & 7`, and a biome tag [U: read as `swamp`; BDS reads a tag id off the level].
- Elsewhere none. sapling draws both rolls from natural spawning's dice.

## Own spawn checks [D]

Each spawned mob's class asks `checkSpawnRules` (actor vtable slot 145, +0x488; classes tied to
vtables through the factory registrar 141e9b120) after the pack's rules and the cluster's caps.

- Monster (14298fc00): `isDarkEnoughToSpawn` (14298f930), then Mob's. In the overworld and the End
  (Dimension +0x1a6, the ultrawarm flag, clear) the block light at the feet (fcn.1415f7530, the
  sub-chunk's block nibble) must be exactly 0, then the brightness recomputed for thunder (sky
  darkening 10), then raw under 8; the Nether asks only raw under 8. Older worlds take other
  branches behind base-game-version gates (140247750 with constants 14cc48300, 14cc486c0,
  14cc486a0, filled at run time) [U]. Zombie and zombie villager (14299de30): the same.
- Zombie pigman (1429904c0): no darkness, difficulty or floor test of its own; refused if any
  entity in its box (fcn.1415fa820) not removed obstructs spawning (Actor +0x26c, set for every mob
  by `Mob::initializeComponents` 1429e0c20), or any block's collision box is in it. Every member of
  a herd is put on the one cell, so the first obstructs the rest: BDS's natural pigmen come alone
  though both rules give herds of 2 to 4 (`spawn_zombified_piglin` weight 100,
  `spawn_few_zombified_piglins` weight 1).
- Guardian and elder guardian (1459a02d0): pass on the level random's `nextInt(20) == 0`, else only
  where Java's `canSeeSkyFromBelowWater` fails (141603400: at or over sea level the height map;
  under it, an opaque block that is no liquid between the spot and the sea's surface).
- Their own: silverfish 142997390, shulker 1429959b0, endermite 14599c770, mooshroom 142a4bfe0 (its
  floor block), slime 142999ca0 (with the lambda above); the warden's and two others return true
  (1400a2880).

Which kinds take Monster's check [D] (2026-11-05): the vtables whose slot 145 is 14298fc00, tied to
ActorTypes through the factories 141e9b120 registers. Monster's own vtable (14a7b7ba0) serves the
zoglin, warden, breeze and creaking; the others are the skeleton class (skeleton, stray, wither
skeleton, bogged, parched), spiders and cave spiders, creeper, enderman, witch, vindicator, evoker,
vex, ravager, wither and ender dragon, each with Monster's `isDarkEnoughToSpawn` (slot 174, +0x570).
The piglin and brute, pillager, blaze, ghast and magma cube take Monster's check with an
`isDarkEnoughToSpawn` returning true (1400a2880): no darkness test. The phantom is not among them
(its own check, "Phantoms").

Sapling: spawning/gates.rs `own_check`, from herds.rs `spawn`, so for the structure pass too: the
pigman's and the guardians' checks, and block light 0 outside the Nether for the monster category
but the kinds above without the darkness test and those with their own check (`OWN_CHECKS`; the
phantom was taken as a Monster, the warden as exempt, before). Tests
`a_dimly_lit_floor_spawns_no_monster`, `guardians_appear_and_stay_in_a_monument`.

The cap before each member [D] (2026-11-05): both readings were right, for different passes.
`_spawnMobCluster` (1415ed230, natural clusters) asks `0 < cap && count < cap`, a category stopping
at its cap; the structure pass in `BedrockSpawner::tick` (1415eebf0, the 1415ef8b2 compare) asks
`-1 < cap && count <= cap`, one over. Sapling: `spawning/herds.rs` `cluster` stops at the cap (it
ran one over before), `spawning/structures.rs` keeps one over.

## Phantoms [D]

`InsomniaSystem::tick` (`148489af0`, per player, not spectators): a sleeping player's count is set to
1; any other's goes up by one while `doInsomnia` (game rule 24) is on and is set to 1 while it is
off. It is saved as `TimeSinceRest` (int; loaded at `141bf7240`). The phantom's pack rule (surface,
light 0 to 7, monster biomes, 5 a cap) finds the spot as for any mob; the phantom's own spawn check
(`14298ff30`) then runs a lambda (`1429ab450`) over the players, which takes the first that:
has the component; is within 100 (squared distance under 10000) of the spot; stands at or over its
column's height map; passes `nextInt(TimeSinceRest) >= 72000` (3 days). The phantom moves to the
player's position plus `nextInt(21) - 10`, `nextInt(15) + 20`, `nextInt(21) - 10`, floored, if
fewer than 5 phantoms are within 48 on each axis, and then `Mob::checkSpawnRules` there.
sapling (`spawning/phantoms.rs`): the same, players in join order, the height map as the highest
solid block [U], the new place only needing room for the phantom, at the cell's inset like any spawn.

## Despawning

BDS's `DespawnSystem` runs `_canDespawn` every tick [D]: no player at all, nothing goes; a
`PersistentComponent` never goes; the soft rules all together (inactive 30 s, a roll of 800, no player
within `min_distance`) despawn; at tick range 4, no player within `min(max_distance, 44)` despawns; and
the simulation edge. Schema defaults [D]: inactivity 30, chance 800, all three on. sapling: a mob
despawns only if it appeared naturally (`spawning::Natural`) and its kind has `despawn_from_distance`
[P]; the roll is 1 in 800 [U], the distances 32/128 are the
add-on documentation's [U], and the edge is no player's view covering its column. Mobs spawned by
commands, eggs or plugins stay.

Saved [D, S]: BDS keeps `Persistent` (its `PersistentComponent`, which `_canDespawn` asks) and
`NaturalSpawn` with every mob; a summoned one is saved `Persistent` 1, `NaturalSpawn` 0
(bds-keepmobs.txt, bds-t2ekeep-dump.txt, the template actors of bds-t12actors). A natural one has no
`Persistent` key and `NaturalSpawn` 1 (`h3keep`, 2026-11-05: the world dumped after 90 s at the swamp
hut at midnight, 31 natural mobs so, the summoned zombie and the hut's witch `Persistent` 1,
`NaturalSpawn` 0). Sapling (`spawning/persistence.rs`) saves a `Natural` mob so and any other
`Persistent` 1, and a mob read back with `NaturalSpawn` 1 and no `Persistent` is `Natural` again; one saved with no `NaturalSpawn` (sapling's worlds before) stays, as it did. Tests
`a_mob_saved_as_natural_comes_back_despawning_and_a_persistent_one_stays`,
`natural_mobs_are_saved_not_persistent_and_summoned_ones_persistent`.

Filter-only despawns [P, D]: where a kind's `minecraft:despawn` has `filters`, `_canDespawn` runs
them instead of its own rules (0x141ba30e0: after the any-player check and a loaded-floor check,
`fcn.141cf1b50` on the filter group; no persistence, view edge or 44 cap of its own). The piglin
brute's and the zoglin's: not `is_persistent` and over 54 from the nearest player, or
`inactivity_timer` 30, `random_chance` 800 and over 32 (sapling `spawning/despawn.rs` `FILTERED`, for
`Natural` mobs of the two kinds). The wandering trader's: the `wandering_trader_despawning` family
(its `minecraft:timer` of 2400 or 3600 s adds it) or no trade supply, and over 24, whatever its
persistence: built before, villagers/wandering.rs (looked at every 20 ticks and not while trading, and
the timer is not saved).

Inactivity and the 44 cap (2026-11-04, bdsre, settled by decompile; sapling `spawning/despawn.rs`):

- The soft rules (0x141ba3960) read the mob's `NoActionTimeComponent` against `min_range_inactivity_timer`
  × 20, then roll the chance, then look for a player within `min_distance`; one found sets the count to
  0 (helper vtable 0x14a774710 slot 7, 0x141ba3940). Sapling counted ticks since the spawn.
- The count goes up 1 a tick for every living mob that ticks (`PostAIUpdateSystem`, 0x1436a08c0) and
  2 more for a `Monster` brighter than 0.5 at its feet plus 0.66 of its height, a blaze and a magma cube
  always (`MonsterAiStepSystem::tickMonsterAiStep`, 0x141c652d0); `Mob::_hurt` sets it to 0. Sapling
  keeps it in `Health` (`idle`), the monster test as the `monster` family and raw brightness 13 or more
  (0.5 on the Java game's light ramp [U]).
- The far cap: `if (level->getChunkTickRange() <= 4) far = min(max_distance, 44)` (0x141ba3a80, ILevel
  slot 14); at a larger `tick-distance` the far distance is the pack's (128). Sapling kept 44 at every
  distance; test `natural_mobs_70_off_go_at_tick_distance_4_and_stay_at_6`.

Persistence by a player [D]: `Player::interact` (1402b7be0) gives the target `PersistentComponent`
after any interaction that took the click, when the target is a `Mob` (Actor+0x210 bit 2): a name
tag, a lead, food, taming, a bucket. The summon command's lambda (1423f1a60) also makes its mob
persistent. Sapling drops `Natural` from a mob any of its click handlers took (`actions.rs` `keep`);
test `a_natural_mob_a_player_names_stays_when_the_player_goes`. Picked-up items were not looked at.

## Left

- Structure spawns and patrols: docs/research/structure-spawns.md (built 2026-11-05: the saved
  record is chunk tag 0x77 with structure names, not tag 57 with type bytes; patrols built
  2026-11-05, not drilled spawning naturally).
- Herd events (spawner blocks: docs/research/spawners.md): undrillable live, natural spawning takes a
  random spot and time; drilled by summoning with the events instead (husbandry.md "Herds").
- `doMobSpawning` and peaceful (2026-11-04, `e1spawn`, bds-e1spawn.txt): with the rule off a monster
  spawner made a pig by its egg still spawned pigs, and a cow egg hatched; `/difficulty peaceful`
  removed a zombie in its own tick and left a pig, and `/summon zombie` then answered
  `commands.summon.failed`. Sapling's run (sapling-e1spawn.txt): the same, answer for answer.
- Snow cover (`is_snow_covered`): `FilterTestBiomeSnowCovered` (0x1430a80b0) reads the biome's
  `mMinSnowLevel` (+0x28) `> 0`, the pack climates whose `snow_accumulation` starts above 0; sapling
  read the `frozen` tag, which misses the snowy taiga and the grove. Fixed: `spawn_rules/biomes.rs`
  `SNOW_COVERED`, test `filters_test_the_biome_tags`.

## Spawn eggs [U]

A spawn egg used on a block hatches its kind (the egg's name less `_spawn_egg`; `evoker`, `tropical_fish`,
`villager` and `zombie_villager` renamed) at the centre of the cell before the face clicked, whatever the
light and rules; spent outside creative. Used on a monster or trial spawner it sets the spawner's
kind instead (docs/research/spawners.md).
