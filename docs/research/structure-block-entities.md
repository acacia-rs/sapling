# Block entities of generated structures (BDS)

What BDS writes into the block entities its generator leaves (spawners, loot containers, trial
spawners, vaults, pots, suspicious blocks), how sapling makes the same, and what is left. Decompiled
from the bds-molang build (`BDS_EXE=acacia/.testserver/bds-molang/bedrock_server.exe`); saved NBT read
off the oracle world `20261003-structures` (testbox `~/oracle-worlds`) with
`cargo run -p sapling-leveldb --example block_entity_dump -- <world>`. [U] = unverified.

## What BDS saves

Every compound has `BlockEntityVersion` Int 0, `id`, `x`, `y`, `z`, keys in sorted order.

| Block | `id` | Fields at generation |
|---|---|---|
| mob_spawner | `MobSpawner` | `Delay` 20, `MinSpawnDelay` 200, `MaxSpawnDelay` 800, `SpawnCount` 4, `MaxNearbyEntities` 6, `RequiredPlayerRange` 16, `SpawnRange` 4 (Shorts); `DisplayEntityHeight` 1.8, `Width` 0.8, `Scale` 1.0 (for every mob); `EntityIdentifier` |
| chest, trapped chest, barrel | `Chest` / `Barrel` | `Findable` Byte 0, `Items` [], `LootTable`, `LootTableSeed` Int |
| dispenser | `Dispenser` | `Items` [], `LootTable`, `LootTableSeed` |
| decorated_pot | `DecoratedPot` | `animation` Byte 0, `item` (empty stack), `LootTable` + `LootTableSeed` when it has loot, `sherds` (4 item names) when not plain |
| suspicious sand/gravel | `BrushableBlock` | `LootTable`, `LootTableSeed`, `brush_count` 0, `brush_direction` Byte 6, `type` (the block) |
| trial_spawner | `TrialSpawner` | `normal_config`, `ominous_config` (the template's strings), `cooldown_end_at` 0, `next_mob_spawns_at` 0, `current_mobs` [], `registered_players` [], `required_player_range` 14, `total_mobs_spawned` 0; `spawn_data` appears once it ticked |
| vault | `Vault` | `config` {`activation_range` 4.0, `deactivation_range` 4.5, `key_item` stack, `loot_table`, `override_loot_table_to_display` ""}, `data` {`items_to_eject` [], `rewarded_players` [], `state_updating_resumes_at` Long (the level time)} |

Tables are pack paths (`loot_tables/chests/simple_dungeon.json`); a template's Java table
`minecraft:chests/x` becomes `loot_tables/chests/x.json`. BDS also writes beds, flower pots, cauldrons,
sculk blocks and the mansion's pre-filled chests (template `Items`), left out here.

## Where the tables and seeds come from

- Dungeons (`MonsterRoomFeature::place` 149e9dae0): the chest's `setLootTable` (144878700: table string at
  +0xf0, seed at +0x110) with `simple_dungeon.json` and the feature generator's `nextInt()` (vt+0x10:
  `next_u32 >> 1`); the spawner's mob is the table at 14a98ef10 by `nextInt(4)`: skeleton, zombie,
  zombie, spider. Both draws happen only when the block actor exists, which it always does.
- Buried treasure (148312c40), igloo (14836fd90: the `chest` marker gives the chest below
  `igloo_chest.json` and a draw), shipwreck markers, desert pyramid, jungle temple (chests and
  `dispenser_trap.json` dispensers), strongholds, fortresses: each chest's seed is the draw sapling
  already made for it (`nextInt()` of the structure's generator).
- Template chests (`placeInWorld` 141eed154: a chest's block actor draws its seed from the piece's generator).
- Pillager outposts (template chests placed without a draw in sapling), ocean ruins (`underwater_ruin_big/small.json`, archaeology `warm_/cold_ocean_ruins_brushable_block.json`),
  mansions (`woodland_mansion.json` from the `Chest*` markers), end cities (`end_city_treasure.json` from
  the `Chest` markers): BDS's seeds come from draws sapling does not model; sapling hashes the cell [U].
- Trail ruins archaeology: the packs' processors give each capped `suspicious_gravel` rule an
  `append_loot` table (`trail_ruins_brushable_block_common/rare.json`; `pools_v2.txt` `c` lines, from
  `jjson.py`).
- Trial chambers and other v2 templates: their NBT (`templates_to_rust.py` `block_entity`, templates.bin
  special kind 4) gives the tables, pots' sherds, trial spawners' configs and vaults' tables and keys;
  BDS's seeds come from a generator sapling does not model; sapling hashes the cell [U].

## Checked

`cargo run --release -p sapling-worldgen --example block_entity_diff -- 20261003 20261003-structures`
(testbox) over the 173 chunks where BDS saved such an entity: every spawner's mob (but four mineshaft
spawners whose block itself differs in the oracle already: 3 BDS has where ours is air or web, 1 the other way),
every trial spawner's config, every vault's table and key, and every table match; the seeds match for
dungeons (42), buried treasure, desert pyramids and their archaeology, jungle temples, strongholds,
igloo, shipwreck, ruined portal, ancient cities and most village chests. They differ where sapling
hashes the cell (trial chambers, mansions, camps, ocean ruins, trail ruins, the outpost) and for 3 of 16
village chests. The desert well is not ported (no blocks, no `desert_well_brushable_block.json`).
Blocks are unchanged: no block write and no generator draw was added or moved, and
`tools/oracle.sh <name> blocks_match_bds_structures` gives the same per-region counts as before.

## Loot on first open

`RandomizableBlockActorContainer` (load 1448783f0) keeps `LootTable` and `LootTableSeed` until the
container is first opened, when the table is rolled into it and both fields go. Sapling rolls it when
the shard first holds the container (opened, fed by a hopper, broken: `containers/loot.rs`), with
`sapling_data::table_loot` and dice seeded from `LootTableSeed`, into shuffled empty slots.

## Left out

- Decorated pots' and suspicious blocks' loot is kept but never rolled (no breaking-pot loot, no brushing).
- The mansion template chests' own `Items`; flower pots, beds, cauldrons and the other non-loot entities.
- A vault's `state_updating_resumes_at` is 0 (BDS: the level time at generation).

## Unverified

- [U] BDS splits a rolled stack over several free slots as Java's `LootTable.fill` does; sapling puts
  each stack in one slot. Whether a comparator or a hopper below unpacks the loot as opening does.
- [U] The seeds sapling hashes (above), and 3 village chests' seeds (some draw of the piece's generator
  before them is not the one BDS makes, though the blocks match).
