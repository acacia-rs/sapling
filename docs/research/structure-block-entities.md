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
`minecraft:chests/x` becomes `loot_tables/chests/x.json`. BDS also writes beds, bells and cauldrons
(sapling too since 2026-11-03, below), flower pots, sculk blocks and the mansion's pre-filled chests
(template `Items`), the last three left out here. Seen in the oracle
world `20261003-structures` (`block_entity_dump`, 2026-10-29): `Bed` `color` Byte (0, 4, 10, 11, 13, 14),
`Bell` `Direction` Int 0, `Ringing` Byte 0, `Ticks` Int (24, 33 seen; ticks loaded, below), `FlowerPot`
`PlantBlock` {name, states, version 18168865}, `Cauldron` `Items` [], `PotionId` (-1, the witch hut's
drawn) and `PotionType` Short -1.

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
the desert well's two match too since its port (worldgen-bds-features.md, "Desert well"). Before the draws
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

## Brushing

Captured on BDS (`research/captures/bds-brush.txt`, `examples/bds_loot_probe.rs brush`) and decompiled;
sapling: `blocks/brushing.rs`, tests `tests/integration/brushing.rs`.

- Input: `PlayerAction StartItemUseOn`, a ClickBlock with trigger PlayerInput, then one ClickBlock a tick
  with trigger SimulationTick (prediction Failure), then `StopItemUseOn`. PlayerInput clicks alone do
  nothing (`examples/shared/brush_hold.rs` sends the held form).
- Brush use-on (147a943b0): a cell with a `BrushableBlock` actor is brushed (141f23fa0); any other
  plays sound `brush` at the centre, extra the block's first-state hash, at most every 9 ticks
  (per-player cooldown: play if cooldown < now, then cooldown = now + 8).
- A stroke: refused while now <= next_due; `brush_direction` 6 takes the face; next_due = (next_due or
  now) + 10, so strokes owed after a pause come one a tick; the loot is rolled into `item` (Count,
  Damage, Name, WasPickedUp) at the first stroke and `LootTable`/`LootTableSeed` removed; count + 1;
  a tick is queued at +40 and the block marked brushed until now + 40.
- Queued tick (141f25300), not deduplicated: still brushed → again at +20; else count 0 → direction 6,
  stop; else count - 2 (floor 0), again at +10. The capture's first fall came ~49 ticks after the last
  stroke, the next 9 later.
- `brushed_progress` follows the count (slot 52, 141f25530): 0 at 0, 1 at 1-2, 2 at 3-5, 3 from 6;
  BlockEntityData goes out only with those UpdateBlocks.
- The tenth stroke (141f241c0): LevelEvent 2021 at the cell corner (data the current hash), the cell
  set to sand, gravel or air, then the item at the neighbour on the brushed side (+0.5, no motion)
  and sound `brush_completed` at the centre; the brush takes 1 damage. Seeds 1461741536 (desert
  pyramid sand) and -1306338408 (gravel) gave `archer_pottery_sherd` and `yellow_dye`, as sapling rolls.

## Left out

- A decorated pot broken by hand rolls its table into its one slot as a container does and leaves that
  beside itself (`blocks/entities.rs`; the probe's pot of seed 1434522187 left item 307, one, as
  `containers::loot::tests` expects). Broken in creative, BDS's pot of that seed left item 307 and not
  the pot (bds-loot-probe2.txt). Fixed 2026-11-03 (`blocks/request.rs` `break_block`, test
  `a_seeded_pot_broken_in_creative_leaves_its_loot_and_not_itself`). The earlier test's entity never
  read back because `set_block_entity` kept an entity without `x`/`y`/`z`, which the column drops
  (`pistons::at_place` now adds them). A shot pot was not drilled.
- Brushing (2026-11-04, `bds_command_probe t3brush`, bds-t3brush.txt): BDS sent no dust particles,
  and answered each brushing click with the whole inventory, armour, UI and offhand windows; the brush
  sound's extra data is the block's hash. Sapling resends the four windows (`blocks/brushing.rs`
  `resend_windows`).
- The mansion template chests' own `Items` and the other non-loot entities (flower pots: below). The
  oracle world 20261003-structures holds a mansion (2026-11-04, `block_entity_dump`): of its template
  chests two keep 8 allium in `Items` and 47 are empty with `Findable` 0. Ported 2026-11-05: templates.bin
  kind `chest` for a mansion `Chest` without `LootTable` (`templates_to_rust.py` `chest_item`; the bin's
  states and palette unchanged, only the mansion specials), `BlockEntity::Chest`, written as BDS saved
  them (`sapling-leveldb` `generated_chest`: `Findable` 0, each item `Block` if a block, `Count`, `Damage`
  0, `Name`, `Slot`, `WasPickedUp` 0, `tag` 0 when empty). The templates hold 69 empty chests and four
  with items: 1x1_b5's eight allium (the oracle's), 1x2_a4's dark oak saplings, 1x2_a6's iron axe (Java
  efficiency, Bedrock ench 15) and 1x2_s2's trapped chest of two ender pearls; [U] the last three's forms.
  Beds, bells and cauldrons are written since 2026-11-03 (templates.bin kinds `bed`, `bell`, `cauldron`;
  the witch hut's cauldron in `scattered/hut.rs`). `block_entity_diff 20261003 20261003-structures`:
  every bed (0, 4, 10, 11, 13, 14), bell and cauldron same. BDS saved the trial chambers' magenta and
  brown beds red (14), which the script copies. No template holds a plain pot.
- The witch hut's cauldron (`SwamplandHut::postProcess` 14836de70, 2026-11-03): after the level's
  `nextInt(3)` the hut draws `nextInt(100)` and sets the cauldron actor's potion id (+0x288, saved
  `PotionId`; `PotionType` stays -1): below 10 water breathing (19), 15 fire resistance (12), 30
  swiftness (14), 40 slowness (17), 65 poison (25), 75 weakness (34), else healing (21). The
  globals (14ccc09f8 ...) were named by the potion registry that stores them (14222a750). Sapling:
  `scattered/hut.rs`, test `the_cauldrons_potion_follows_bds_bounds`. The oracle's one hut
  (3844 72 -1818) saved `PotionId` 25, `PotionType` -1; `block_entity_diff` same.
- Bells' `Ticks` (2026-11-03, `block_entity_dump` of both oracle worlds): the same two bells saved
  24/33 in 20261003-structures and 21/21 in 20261003-structures-still, so it counts the ticks the
  bell was loaded, not anything generation draws. Since 2026-11-04 sapling counts every bell's `Ticks`
  while its column ticks, wrapping at 50 (`blocks/bells.rs`; bds-t3bells.txt's copies of a bell held
  its count since placing, mod 50), so a generated bell counts once loaded as BDS's did.
- Flower pots (2026-11-03): BDS saved 46, every one `FlowerPot` {`PlantBlock` {name, `states` {},
  `version` 18168865}}. The witch hut's (block `update_bit=1`) holds `minecraft:red_mushroom`;
  sapling writes it (`hut.rs`, `generated_flower_pot`), `block_entity_diff` same.
- Template flower pots (2026-11-03, `block_entity_dump` of 20261003-structures): the 45 others hold
  allium 11, deadbush 22, white tulip 5, red tulip 4, oxeye daisy 2 and cactus 1. The tulips, allium
  and daisy are the mansion's: its 1.11 templates save a legacy `flower_pot` whose block state
  (`contents=cactus` or `empty`) says nothing, and a `FlowerPot` entity with `Item`
  `minecraft:red_flower` and `Data` (2 allium, 4 red tulip, 6 white tulip, 8 oxeye daisy; also
  `yellow_flower`, `sapling` 2, `cactus`), which BDS names by the plant's own Bedrock block. Newer
  templates name it in the block (`potted_dead_bush` is `minecraft:deadbush`). `PlantBlock`'s
  `states` are the plant's own: the cactus saved `age` Int 0, the flowers `{}`. Sapling:
  templates.bin kind `flower_pot` (`templates_to_rust.py` `flower_pot_plant`; regenerate it, no
  palette index moved), `generated_flower_pot` writes a cactus's `age` and a sapling's `age_bit`,
  test `templates_keep_their_block_entities`.
- A potted sapling (2026-11-04, `bds_command_probe potsapling`, research/captures/bds-potsapling.txt:
  pots clicked with each plant, the world read with `block_entity_dump`): `PlantBlock` `states`
  {`age_bit` Byte 0} for oak and cherry saplings, as `generated_flower_pot` writes; a mangrove
  propagule {`hanging` Byte 0, `propagule_stage` Int 0}, bamboo {`age_bit` Byte 0,
  `bamboo_leaf_size` "no_leaves", `bamboo_stalk_thickness` "thin"}, cactus {`age` Int 0}, poppy {}.
  Sapling's click (`pots.rs`) keeps the same, test `a_potted_plant_keeps_its_first_state_as_on_bds`;
  sapling-potsapling.txt sent the same `UpdateBlock`s and `PlantBlock`s.
- Block entities on the wire (2026-11-04, every `BlockEntityData` of research/captures/bds-*.txt,
  and `bds_command_probe setshown`, bds-setshown.txt): every one BDS sends, of all 34 ids seen
  (pots, banners, chests, pistons, moving blocks, command blocks, ...), carries `BlockEntityVersion`
  Int 0, and every compound's keys are sorted (BDS's compounds are maps): `{BlockEntityVersion,
  PlantBlock {name, states, version}, id, x, y, z}`. Sapling sent theirs in the order kept, the
  version only where a module wrote it (structure blocks, gateways). `/setblock` of a pot, cauldron,
  skull, beacon, chiseled bookshelf, bell, chest or bed sent the new entity at once (a bed both
  halves, `color` 14: a command's bed is red); sapling made none for a pot, cauldron, skull, beacon
  or bookshelf, and gave a command's bed no entity. Fixed: `BlockEntity::shown`
  (sapling-leveldb) adds the version and sorts, for chunks and `BlockEntityData` alike; blocks/entities.rs
  `furnish` makes and shows those entities (beds through `commanded::put_bed`); tests
  `a_block_entity_is_shown_as_bds_shows_it_versioned_and_sorted`, set_shown.rs. Saved block
  entities are unchanged (sapling's own lack the version on disk). Left then: two block
  differences sapling-setshown.txt showed. BDS's unsupported set frame was gone by its removal
  (`noChange`), and its bed's `setblock … air` sent no UpdateBlock. Drilled 2026-11-04 (bds-e3sets.txt,
  e3mouseframe): a frame set with nothing to hang on shows its break dust at the set and goes, item
  left, a tick later (`set_entities::is_unheld_frame`); a bed half set to air shows the other half's
  `ParticleDestroyBlockNoSound` at its corner, then both halves as air in one UpdateSubchunkBlocks
  (`commanded::dust_bed_half`). Fixed; tests in set_shown.rs and command_fixes.rs.
- Set entities (2026-11-04, `bds_command_probe s7ents`, bds-s7ents.txt against sapling-s7ents.txt):
  39 blocks set one by one. Fixed (blocks/set_entities.rs, `BlockEntity::shown` and `updated`):
  - A set frame, glow frame, lectern, jukebox (`FinishedRecording` 0, `TicksPlaying` 0), lodestone,
    mob spawner (its defaults written out), trial spawner, vault, sign, hanging sign, campfire,
    banner, enchanting table (`rott` 0), decorated pot (`animation`, an empty `item`) and suspicious
    sand get their entity at the set and show it. Sapling made these on use, or not at all.
  - A furnace, hopper, brewing stand or dispenser shows none at a set (sapling showed its
    container), nor do a note block, daylight detector, comparator, beehive, the sculk blocks or a
    creaking heart.
  - A shulker box shows `Findable` 0 and its empty `Items`. A cauldron shows its empty `Items`.
    An empty list is shown with no tag.
  - A sign shows each side's `FilteredText` "" and `LockedForEditingBy` -1, as do signs placed
    any other way (bds-unheld.txt).
  - A `BlockEntityData` of a trial spawner is an empty compound. A vault's is `connected_particle_range`
    4.5, `connected_players` [] and an empty `display_item`. A crafter's is `crafting_ticks_remaining`
    and `disabled_slots`. None carries an id. Chunks still carry the saved compound.
  - A tick later BDS shows a set frame, glow frame, lectern, jukebox (now `FinishedRecording` 1),
    sign, campfire, cauldron, skull and command block again, block and entity. It shows a bed's two
    halves' entities again without their block.
  - Tests `a_set_block_shows_its_entity_as_bds_s_did_and_some_again`,
    `a_cauldron_shows_its_empty_items_and_a_sign_its_wire_fields`.
  - An idle vault is not shown again: sapling's showed its entity at each state update
    (spawners/vault.rs now shows it on a new state or display item alone).
  - Re-drilled (sapling-s7ents.txt): every entity, its fields and its second send as BDS's. Left,
    not block entities: BDS's set daylight detector read 10 some 30 ticks on (sapling's stays 0,
    redstone.md), and its sculk sensor heard the bed's set (phases 1 and 2), sapling's did not.
    Fixed 2026-11-04 (bds-e3sets.txt): a detector set by command waits for the next second's
    reading (`daylight::left_for_the_second`; one placed by hand is read the next tick); the bed's
    second show sends `BlockPlace` from its foot, heard by the sensor, whose power sounds play at
    its corner with id -1 (`shown_again::show_beds`, `sculk::play`).
  - Drilled 2026-11-05 by `/clone` (bds-v2ents.txt, bds-v2ran.txt): a shulker box filled by
    `replaceitem`, a sign written by hand, a vault a key was used on and a trial spawner given an egg
    are cloned with what they hold, shown as BDS's (the box's `Items`, the sign's text, the vault's
    reduced form, the spawner's `spawn_data`), a cloned box marked `forceunpair` 1 as a lone chest is
    (`containers::pairs::carried`). BDS shows a clone's cell in one UpdateSubchunkBlocks listing it
    twice with its layer-1 air as `extra`, then the entity once or twice (twice in 7 of 9 clones);
    sapling's batch showed the new entity, the copied one, the block and the entity again, and now the
    block then the entity (`send::keep_entity` and `set_entities::furnish` leave a held cell's entity to
    `show_held`). Not matched: the doubled listing and the second entity. Neither the vault nor the
    spawner ran a wave in the drill (the vault stayed inactive, the spawner spawned nothing in 60 s):
    a trial spawner needs `doMobSpawning` on, which the drill floor turns off.
  - Clone shape (2026-11-05, bds-x2clone.txt; BDS's clone fcn 14160c130): a move's emptied source
    first, then each copy's block (when it changed) with its source's layer 1 as `extra` (air, or a
    fence's water, listed changed or not), then every copy listed again, in one UpdateSubchunkBlocks a
    sub-chunk; each copy's entity is shown twice after (chest, shulker box, vault, trial spawner).
    Sapling's `/clone` now goes through `ShardCtx::copy_blocks` (`blocks/cloned.rs`); test in
    thirteenth2x.rs. A waterlogged block's water counts in the answer (a fence over water: `["2"]`).
    Re-drilled (sapling-x2clone.txt): every clone's packets as BDS's. Found, not followed: a fence
    `/setblock` over water is listed twice in one UpdateSubchunkBlocks with its final connections on
    BDS; sapling sends two UpdateBlocks, unjoined then joined.
  - A vault and a trial spawner that ran (bds-x2ran.txt, mob spawning on): their cloned entities as
    BDS's but for the vault's `connected_players`; what each showed while running: spawners.md
    "Parity drill (2026-11-05)".
  `block_entity_diff 20261003 20261003-structures`: 46 of 47 pots same; the one missing, a trial
  chambers' dead bush at (-446, -11, -425), is water in our blocks there (a block difference).
  Looked at 2026-11-05 (thirteenth3w, `cells 20261003 20261003-structures[-still] -448 -12 -432 -433 -9
  -417`): ours has water sources in a strip two wide along the chunk's west edge (x -448 and -447, y -11,
  all 16 z) and flowing water a cell or two further east at five z, the pot's cell among them; both BDS
  worlds, stock and tick-free, have air there and the pot. The strip enters from chunk -29, which
  neither BDS world saved, so its source (a corridor flooded in our chunk -29, sources made by the
  two-source rule as it spreads) needs a BDS run that saves chunk -29 to be judged.
  Judged 2026-11-05 (thirteenth3y) on `20261003-y3tc` (BDS 1.26.52.3, server-side generation, regions
  -30, -28, -27 and -26 saved): BDS's corridor is dry with its pot and tripwires. Ours waterlogged the
  waxed copper grates at y 0 (x -454 and -453, z -423 to -420) that the chamber sets into an aquifer;
  their water fell to y -4 and ran east into the corridor. BDS's grates hold none: trial chambers
  ignore waterlogging, as Java's `liquid_settings` `ignore_waterlogging`. Fixed: `Canvas::dry` for
  trial chamber pieces (test `trial_chamber_blocks_set_into_water_keep_none`); `cells` over x -460 to
  -433, y -14 to 2 then differs only in two ore blobs (gold at x -436 to -434, y -6 to -4; copper at y 20
  earlier), not looked into.
  Looked at 2026-11-05 (thirteenth3ab): a second BDS run of the same regions (`20261003-y3tc-run2`,
  `worldgen-oracle diff`) differs from the first in 18 cells of glow lichen and tripwire, none of them
  ore, so the blobs are not BDS's run spread between these two runs. `cells` over all of chunk (-28, -27),
  y -64 to 120: 52 cells differ, every one an ore blob. Copper at x -446 to -444, z -420/-419, y 20-21
  is BDS's alone (andesite in ours); gold at x -436 to -434, z -426 to -424 sits a block lower in ours
  (y -5/-6 against -4/-5); coal on the chunk's north and west edges (z -432, x -448) at y 31-32 in ours
  and 29-30 in BDS; gold on the north edge at y -40/-41 in ours, -42 in BDS. Same x and z, y off by one or
  two: the blobs' heights, not their draws across; the edge ones may come from the neighbours (-28, -28)
  and (-29, -27). No repeatable canon covers this chunk (`c-tc` is chunks 2-14, 12-24), so not traced;
  next: a one-worker seed-0 BDS run of these chunks and `replay_check` (memory: stock twins agreeing
  can be luck).
  Judged 2026-11-05 (thirteenth3ad): with Linux generation (`SAPLING_ORACLE_LINUX=1`) `cells` still gave
  the same 52, so not the rule order. A one-worker seed-0 Linux run of chunks -30..-26, -29..-25
  (canon `tcores`, dir `ad3tc`, worldgen-bds-linux-canons.md): `replay_check` 0 cells differ in each
  source chunk writing into (-28, -27), and `blocks_match_bds_canons`' new region "trial chamber ores"
  (that chunk) is 0 wrong, 0 missing. The blobs' heights were the stock runs' own (both stock runs drew
  them alike), not ours.
- A vault's `state_updating_resumes_at` is 0 (BDS: the level time at generation); nothing reads it
  before the vault's first tick rewrites it (`spawners/vault.rs` takes the lesser of it and now + 1
  step), so left. Undrillable as a difference: the generator has no level clock to save, and a loaded
  vault steps at its first tick either way (2026-11-04).

## Unverified

- [U] Brushing's catch-up rate: the decompile gives one owed stroke a tick (sapling's), but the
  capture's gravel brushed again after a pause showed progress 1, 2, 3 and done ~5, 10, 16 ticks in
  (one a tick would be 2, 5, 9). Probe a second pause with tick stamps to settle it.

- `enchant_randomly` (a dungeon's book; thirteenth2d, 2026-11-05): BDS's `EnchantRandomlyFunction`
  (apply 14752bc20) draws one `nextInt(30)` from the loot random as the cost, then enchants by
  `EnchantUtils::randomlyEnchant` (141b62a20) on a thread-local random, `treasure` as the JSON says.
  Sapling does the same (`sapling-data` `mob_loot/item_data.rs`, its enchantments unrepeatable);
  seeds 5 and 73, books rolled third and first, fill BDS's slots (bds-loot-books.txt; test
  `a_dungeon_book_draws_its_cost_and_the_chest_stays_in_step`).
- [U] Whether a comparator reads 0 from unopened loot or the drill's comparators faced the wrong way.
- 3 village chests' seeds (settled below: same under the run's order): BDS's are draws 157 of chunk (-63, 9)'s structure MT and 306 and 459 of
  (-62, 8)'s, sapling's 158, 301 and 449, while (-62, 8)'s chest at draw 433 matches; the draws
  between are the pieces' bottom fills (`nextInt(3)` per filled column over the whole piece), so some
  fill reads a column outside the chunk differently from BDS, though the blocks match. Not the
  stock oracle's chunk order: against the repeatable c-vi replay (`ENVS="$(tools/replay-env.sh
  c-vi)"`, 2026-11-05) the same three differ. The fill block test (141f7a9c0, read 2026-11-05) skips a
  top block whose `BlockType+0x130` has bit 0x20 (the leaves property) and else takes the biome's top
  material, as sapling's `fill_below`; so the draws' difference is in which tops are not air, not there.
  Traced 2026-11-05 (thirteenth3ah): a Linux canon run of -66,4..-59,11 under gdb with a breakpoint on
  the fill's `nextInt(3)` (Linux 0xd2bda58: BlockPos at rsp+0x10, the chunk box at r12), against
  sapling's c-vi replay printing each fill (a temporary trace in `jigsaw/piece.rs` `fill_bottom`). The
  Linux run's post order is not Windows' (so its counts are no check on the Windows canon), but the
  rule shows. BDS's fill reads the out-of-chunk columns of a piece as they stand. Before the owning
  chunk posts, terrain is there and is filled (piece (-996, 86, 141)..(-990, 90, 146): row z 146,
  x -996..-993, lies in chunk (-63, 9) and was drawn from the posts of (-62, 9), (-63, 8) and (-62, 8)).
  Once the owning chunk has placed the piece's air there, the column is skipped. Sapling read air at
  those four cells from every chunk, so (-62, 8) drew 4 fewer, and read grass at (-990, 86, 146)
  (chunk (-62, 9)) from (-63, 8) and (-63, 9) but air from (-62, 8). In (-63, 9) it also filled
  (-1013, 86, 148..154) (chunk (-64, 9)), which BDS never did (6 more). So sapling's view of a
  neighbour's not-yet-posted column during a post is the thing to fix. It is not the fill or the
  block test. Not reached: where sapling's canvas gets those cells (`structure/` post view).
  Settled 2026-11-07 (thirteenth3aj): wrong lead. `block_entity_diff` built its generator without
  `oracle_support::replay`, so the c-vi ENVS never applied and every run above was the canonical
  order: there chunk (-63, 9) posts first of its four (writers traced: its own post put the piece's air
  at (-996..-993, 86, 146) before (-63, 8), (-62, 9) and (-62, 8) read it, all from grass in the carved
  terrain). With the replay applied (fixed in the example), c-vi posts (-63, 8) and (-62, 8) before
  (-63, 9), as BDS, and all 12 village chests' seeds match (`ENVS="$(tools/replay-env.sh c-vi)"
  tools/remote-example.sh vi block_entity_diff 20261003 20261003-structures`). Sapling's canvas view
  was right; the stock oracle's 3 seeds are its run order against the canonical one, which the stock
  worlds can't judge.
- The desert pyramid sand at (-6114, 60, -3508), fixed 2026-11-05: BDS's cellar sand (1483669e0)
  draws a seed for every suspicious sand cell in the 3x3 view that holds its block, a neighbour's
  included, so the last chunk to post-process the pyramid sets it (`Canvas::set_entity_in_view`;
  `block_entity_diff 20261003 20261003-structures`: 7 of 7 same).
