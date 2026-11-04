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

Left out: `pillager_patrol` (needs patrol events, world age, villages) and the guardian (ocean
monuments, BDS's structure spawns); the slime and the phantom have their code gates (below); herd events (babies,
horse and fish variants, riders), `disallow_spawns_in_bubble`; blazes, wither skeletons, cats and
pillagers have no natural conditions (structures spawn them).

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
  worlds of 1.21 on. Herd events and guaranteed members are not built.
- Caps: counts over the 9×9 chunks round the cluster's chunk, split surface/underground: per category
  and per kind (`density_limit`, clipping the herd to what is left). The category cap is asked before
  each member, `count <= cap`, so a category ends one over it. Every spawn is also clipped so the world
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
add-on documentation's [U], and the edge is no player's view covering its column. Mobs spawned by commands, eggs or plugins, and every mob loaded
with its column, stay: sapling keeps no `Persistent` flag, so a natural mob saved with its column (a
save while it is near a player) comes back persistent. The filter-only despawns (wandering trader,
piglin brute, zoglin) are not built.

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

- Structure spawns [D, not built]: `BedrockSpawner::tick` (1415eebf0) asks the chunk's hardcoded
  spawn areas (1438cfcd0) at the attempt's cell; with none it runs the cluster above, with one it
  spawns from the area's own list (fortress, monument, witch hut, pillager outpost), the same caps
  asked per member, the members persistent when the area's flag says. Sapling cannot: its worlds
  carry no spawn areas (BDS writes them per chunk with the structures; neither sapling-world reads
  them nor sapling-worldgen writes them), so building it starts with that record. Looked at
  2026-11-04: the areas come from `StructureFeature::addHardcodedSpawnAreas` (its lambda 141f08120,
  vtable 14a784210), kept per chunk as what is taken to be LevelDB tag 57 (a count, then per area six
  ints, a box, and a type byte: 1 fortress, 2 witch hut, 3 monument, 5 outpost, read off the lambda,
  not off a saved world). Not ported: it needs sapling-worldgen to emit each structure's box with its
  start, sapling-leveldb to write and read the record, and the spawner to ask it; the tag and the type
  numbers want a BDS world's chunk dump first.
- Patrols: `pillager_patrol` needs the patrol spawner, world age and village checks; not read.
- Herd events (spawner blocks: docs/research/spawners.md): undrillable live, natural spawning takes a
  random spot and time (husbandry.md "Herds of babies").
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
