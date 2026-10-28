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
- Legacy template chests (`placeInWorld` 141eeb8d0, at 141eed154): a type-2 block actor (chest, trapped
  chest) whose tag has no `LootTableSeed` draws `nextInt()` from the piece's generator; barrels and
  other containers draw nothing and load the missing seed as 0. `TemplateStructurePiece::postProcess`
  (141f797b0) then hands every marker to the structure's handler, with no clipping of its own.
- Ocean ruins: the chest marker handler (148356170) draws `nextInt()` of the chunk's structure MT for
  every `chest` marker before its chunk check. Archaeology (`141eedbb0` after the markers): the capped
  processor (145e12a20) shuffles the candidates on a Xoroshiro of the origin, then each converted
  block draws that Xoroshiro's `nextInt()` (low 32 bits of `nextLong`, lambda 145e2aeb0).
- Mansions (1480b2360): a `Group of Allays` marker draws `nextInt(3)`; a `Chest*` marker inside the chunk
  where no chest stands yet places one and draws `nextInt()` (none where a chest already is).
- Pillager outpost (148359310, after its chunk check): the template's chest draws as above, then the
  `topChest` marker re-seeds the chest below it with `nextInt()` (`setLootTable` 144878700).
- End cities (14831aaa0): a `Chest` marker whose chest below lies inside the chunk draws `nextInt()`
  of the chunk's structure MT [U: no End oracle with block entities].
- v2 templates (trial chambers, trail ruins, camps): `WorldGenerator::postProcessStructures` (14196c5f0)
  builds one Xoroshiro per chunk (`setDecorationSeed` 145941320: the level's Xoroshiro gives
  `a = nextLong | 1`, `b = nextLong | 1`, then `(cx * a + cz * b) ^ level`) and passes it to every v2
  start's pieces in order. Each placed block whose actor is a randomizable container (chest, barrel,
  dispenser, pot, brushable; not a banner: 141ee0510 at 141ee1f71) takes its low 32 bits of
  `nextLong` as `LootTableSeed`, overwriting `append_loot`'s (1459418f0), and an empty barrel draws too.
  Their NBT (`templates_to_rust.py` `block_entity`, templates.bin special kind 4) gives the tables,
  pots' sherds, trial spawners' configs and vaults' tables and keys; trail ruins' capped
  `suspicious_gravel` rules carry `append_loot` tables (`pools_v2.txt` `c` lines, from `jjson.py`).
- `examples/loot_seed_fit.rs` finds which draw of which candidate generator a list of BDS seeds is.

## Checked

`cargo run --release -p sapling-worldgen --example block_entity_diff -- 20261003 20261003-structures`
(testbox) over the 173 chunks where BDS saved such an entity: every spawner's mob (but four mineshaft
spawners whose block itself differs in the oracle already: 3 BDS has where ours is air or web, 1 the other way),
every trial spawner's config, every vault's table and key, and every table match. Every seed matches
(`SEEDS=loot` prints the rest) but 3 of 16 village chests and 1 of 7 desert pyramid suspicious sands;
the desert well is not ported (no blocks, no `desert_well_brushable_block.json`). Before the draws
above were ported, 447 seeds differed. Blocks are unchanged (`tools/replay-all.sh`, nether and end
replays): the new draws are of generators nothing else reads after them.

## Loot on first open

`RandomizableBlockActorContainer` (load 1448783f0) keeps `LootTable` and `LootTableSeed` until the
container is first opened, when the table is rolled into it and both fields go. The unpack (144878e90)
seeds an MT19937 `Random` (vtable 14a7125a0) with the seed (0 takes one from the level's random) and
calls `LootTableUtils::fillContainer` (140af0520), which runs `LootTable::fill` (14127e3b0):

1. Each pool (`LootPool::addRandomItems` 14127b8e0): its conditions; rolls `min + nextInt(max - min + 1)`,
   drawn even for a written number (`nextInt(1)`); a `nextFloat` for bonus rolls; per roll, the entries
   whose conditions hold, `nextInt(total weight)`, the first entry the draw runs out in; `set_count`
   draws as the rolls do.
2. The empty slots in order, shuffled (MSVC `std::shuffle`: slot i swapped with `raw % (i + 1)`).
3. Stacks of more than one set aside. If the slots outnumber the single stacks, one set-aside stack at a
   time (`nextInt(count of them)`) is split (`nextInt(count / 2) + 1` off it), each part of more than
   one set aside again on `nextBoolean`. BDS counts the room once (`[rbp+0x4d0]`), so it splits until
   none is set aside, where Java counts again each time.
4. All stacks shuffled as the slots were, then each put in the slot popped off the back of the list.

Sapling does the same (`containers/loot.rs`, `sapling_data::drawn_table_loot`) when the shard first
holds the container (opened, fed by a hopper, broken). `tools/bds-loot-setup.sh` and
`bds_loot_probe` (research/captures/bds-loot-probe.txt) load seeded blocks on a BDS with
`/structure load loot:row`: a `simple_dungeon.json` chest of seed 357601146 gets the 12 stacks
`containers::loot::tests` expects, slot for slot. A hopper under such a chest unpacks it the same way and
starts pulling; comparators beside one lit nothing before it was opened.

## Left out

- Decorated pots' and suspicious blocks' loot is kept but never rolled (no breaking-pot loot, no brushing).
  Found so far: `BrushableBlockActor` (vtable 14a786450; load 141f24d50, save 141f25010 with
  `brush_direction` +0x344 and `brush_count` +0x340) sets `brushed_progress` from its count (slot 52,
  141f25530): 0 at 0, 1 at 1-2, 2 at 3-5, 3 from 6.
- The mansion template chests' own `Items`; flower pots, beds, cauldrons and the other non-loot entities.
- A vault's `state_updating_resumes_at` is 0 (BDS: the level time at generation).

## Unverified

- [U] `enchant_randomly` (a dungeon's book) draws nothing in sapling; a table that rolls a book desyncs
  after it. Checked on one seed only.
- [U] Whether a comparator reads 0 from unopened loot or the drill's comparators faced the wrong way.
- [U] 3 village chests' seeds: BDS's are draws 157 of chunk (-63, 9)'s structure MT and 306 and 459 of
  (-62, 8)'s, sapling's 158, 301 and 449, while (-62, 8)'s chest at draw 433 matches; the draws
  between are the pieces' bottom fills (`nextInt(3)` per filled column over the whole piece), so some
  fill reads a column outside the chunk differently from BDS, though the blocks match.
- [U] The desert pyramid sand at (-6114, 60, -3508): its seed is not the cellar Xoroshiro's draw sapling
  makes, likely the cell another chunk's post-process placed first (`cellar.rs`).
