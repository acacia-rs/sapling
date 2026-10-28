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
- Biome filters: a list is `all_of`; `has_biome_tag` with `==`/`!=`/`not`; `is_snow_covered` is taken as
  the biome's `frozen` tag. [U]

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
- Herd: size min..max [U: the exact expression], and every member on the same cell, at
  (x + 0.49, y, z + 0.49) in worlds of 1.21 on. Herd events and guaranteed members are not built.
- Caps: counts over the 9×9 chunks round the cluster's chunk, split surface/underground: per category
  (the cluster needs `count < cap`) and per kind (`density_limit`, clipping the herd to what is left).
  Every spawn is also clipped so the world holds at most 200 spawnable mobs (BDS: those ticked last tick).
  The per-category caps were not found (read off the dimension, `region vfunc 0x128`); sapling uses
  monster 8, animal 4, water_animal 5, ambient 2 for both [U].
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
[P]; inactivity is counted from its spawn [U], the roll is 1 in 800 [U], the distances 32/128 are the
add-on documentation's [U], and the edge is no player's view covering its column. Mobs spawned by commands, eggs or plugins, and every mob loaded
with its column, stay: sapling keeps no `Persistent` flag, so a natural mob saved with its column (a
save while it is near a player) comes back persistent. `despawn_from_inactivity` and the filter-only
despawns (wandering trader, piglin brute, zoglin) are not built.

## Left

Per-category caps, the herd size expression, persistence from name tags, taming, leashes and picked-up
items (no `PersistentComponent` seen besides rules' `mPersistence`), structure spawns (fortresses,
monuments, huts: the tick's second part), patrols, herd events (spawner blocks:
docs/research/spawners.md). Peaceful does not remove monsters already there [U]. Whether `doMobSpawning` stops
spawners and eggs was not checked; sapling's eggs ignore it.

## Spawn eggs [U]

A spawn egg used on a block hatches its kind (the egg's name less `_spawn_egg`; `evoker`, `tropical_fish`,
`villager` and `zombie_villager` renamed) at the centre of the cell before the face clicked, whatever the
light and rules; spent outside creative. Used on a monster or trial spawner it sets the spawner's
kind instead (docs/research/spawners.md).
