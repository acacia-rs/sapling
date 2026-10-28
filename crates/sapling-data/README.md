# sapling-data

What a client must receive before it can spawn, as packet templates.

## Provenance

Everything in `data/` is a packet body the vanilla dedicated server (BDS 1.26.52, protocol 2193) sent to a
bot joining a fresh survival world. The recording ships in acacia as
`acacia-testserver/fixtures/bds-1.26.52.script`; `tools/extract-data` copies the bodies out of it:

```
cargo run -p extract-data            # rewrite data/*.bin
cargo run -p extract-data -- --list  # the whole recorded join sequence
cargo run -p extract-data -- --show StartGame,UpdateAbilities
```

Mojang's data is not committed (`.gitignore`); build it before the first `cargo build`:

| Files | Made by |
|---|---|
| `data/*.bin` | `cargo run -p extract-data` (the acacia recording above) |
| `data/mob-loot.json` | `py -3 data/generate/mob-loot.py` (BDS's behaviour packs, `PACKS` at the top) |
| `data/block-loot.tsv` | `py -3 data/generate/block-loot.py` (the extracted Java 26.3 client jar, `JAVA` at the top) |
| `data/names.tsv` | `cargo run -p sapling --example bds_name_probe -- <your BDS> > data/names.tsv`, on testbox `tools/remote-drill.sh bds bds_name_probe data/names.tsv` (BDS's own answers to `/give` and `/kill`); optional: without it names are made from identifiers |
| `data/command-names.tsv` | `cargo run -p sapling --example bds_enum_probe -- <your BDS> > data/command-names.tsv` (drop a trailing `probe exit` line when made by `tools/remote-drill.sh bds bds_enum_probe`): the `Item` and `Block` enums of BDS's AvailableCommands; optional: without it commands take every name |
| `data/spawn-rules.tsv`, `data/despawns.tsv` | `py -3 data/generate/spawn-rules.py` (BDS's behaviour packs, as `mob-loot.py`) |
| `data/husbandry.tsv` | `py -3 data/generate/husbandry.py` (BDS's behaviour packs, as `mob-loot.py`) |
| `data/trades.json` | `py -3 data/generate/trades.py` (BDS's behaviour packs, as `mob-loot.py`) |
| `sapling-worldgen/src/structure/template/templates.bin` | `py -3 crates/sapling-worldgen/scripts/templates_to_rust.py <your BDS dir>` |

| File | Sent as | Patched per join |
|---|---|---|
| `StartGame.bin` | `start_game(&StartGameOptions)` | ids, game mode, position, world spawn, world name, tick, seed (0), `client_side_generation` (off), `block_network_ids_are_hashes` |
| `ItemRegistry.bin`, `BiomeDefinitionList.bin`, `AvailableEntityIdentifiers.bin`, `FeatureRegistry.bin`, `CameraPresets.bin`, `CreativeContent.bin`, `TrimData.bin`, `CraftingData.bin` | `registry_packets()`, byte for byte (BDS sends CraftingData later, after the player's own windows) | nothing |
| `JigsawStructureData.bin`, `VoxelShapes.bin` | `pre_start_packets()`, byte for byte, sent before StartGame (vanilla kicks with "missing structure data" without the first) | nothing |
| `UpdateAttributes.bin` (the last one before spawn: the full list) | `update_attributes(id)` | runtime id |
| `UpdateAbilities.bin` | `update_abilities(id)` | unique id |
| `SetEntityData.bin` (the player's own) | `entity_data(id, name)`, `player_metadata(name, flags)` | runtime id, nametag, extra flags |
| the two above | `add_player(&AddPlayerOptions)`: a player for its viewers (shape from `bds_probe`, DESIGN.md "Step 3 as built") | identity, position, flags, device |

BDS sends more before spawn (PlayerList, AvailableCommands, UnlockedRecipes). sapling does not replay
those: they are world state or features later steps own. `--list` shows where they sit.

## Mobs

`Mobs.bin` and `ItemEntity.bin` come from a live BDS instead of the join recording: the `sapling` example
`bds_mob_recorder` starts the server, summons every summonable kind beside a bot (and again with
`minecraft:entity_born`), and keeps the AddEntity each is announced with, then one AddItemEntity from a
loot drop. `Mobs.bin` is those bodies one after another, each behind a little-endian u32 length.

```
cargo run -p sapling --example bds_mob_recorder -- <BDS folder> <its address> crates/sapling-data/data
```

| File | Sent as | Patched per mob |
|---|---|---|
| `Mobs.bin` (112 kinds, 40 of them also as a baby; the villager and the zombie villager, which BDS announces as `villager_v2` and `zombie_villager_v2`, were added by `bds_kind_recorder`) | `add_entity(&AddEntityOptions)` | ids, position, rotation, velocity (none), `Variant`, `MarkVariant`, `Color`, name tag, the baby flag, extra flags, health |
| `ItemEntity.bin` | `add_item_entity(id, &Stack, position, velocity)` | ids, position, velocity, the stack (network id from `ItemRegistry.bin`) |

A kind's hitbox, attributes and flags are whatever BDS gave the summoned one. `minecraft:villager` and
`minecraft:zombie_villager` are not recorded (BDS announces their v2 kinds instead) and are sent as
those. Any other unrecorded kind gets a plain 0.9 x 0.9 mob without attributes. A tadpole is only ever
a baby. `add_painting`, `mob_armor` and `mob_hand` are built, not recorded: their shapes are what
`bds_entity_probe` prints for a world with a stored painting and armour stand (`sapling-leveldb` example
`rewrite`).

## Items

`items.rs` reads two of the recorded registries: item names to network ids (`ItemRegistry.bin`) and the
creative inventory's entries by the id a client picks them with (`CreativeContent.bin`). Reading item
stacks needs acacia-proto to know which item is the shield: `set_shield_item_id()`, once per process.
`inventory_content` and `player_hotbar` have the shape of BDS's join packets
(`extract-data -- --show InventoryContent,PlayerHotbar`). `max_stack` is a list of name endings, not data.

`recipes.rs` reads the crafting grid's shaped and shapeless recipes out of `CraftingData.bin`, by the
network id a craft request names: what each cell takes and what a craft makes. `recipes/smithing.rs`
reads the smithing table's transform and trim recipes from the same file and the trims' patterns and
materials from `TrimData.bin`. The other workstations' rules are `enchant/{table, anvil, grind}.rs`,
over each enchantment's BDS numbers (`enchant/kinds.rs`) and what each item takes (`gear.rs`;
docs/research/workstations.md).

## Item tags and block loot

Neither travels in a packet, and BDS ships neither as files (item tags are compiled in; of block loot
only 99 tables of 1.26.50's new blocks). Both are made by the scripts in `data/generate/` from the Java
26.3 client jar, with items renamed to Bedrock's and checked against `ItemRegistry.bin`'s names;
`data/generate/loot-and-tags.md` has the formats, the row counts and every caveat.

| File | Read by | What |
|---|---|---|
| `item-tags.tsv` | `recipes.rs` | the members of the 17 tags recipes name; 225 of 238 rows also confirmed by pmmp/BedrockData's dump of BDS 1.26.30 |
| `block-loot.tsv` | `block_loot.rs` | one row per thing a block may leave, for the 546 blocks that do not simply leave themselves: tool condition, Java block states, count range, chance by Fortune level, Fortune formula |
| `block-loot-nothing.txt` | `block_loot.rs` | blocks that leave nothing; only those with an empty or missing table are read (the others have rows) |

Rows keep Java's block ids; `java_block` maps a Bedrock block name to one (a hand-written list of the
renamed blocks, and `*_double_slab` to its slab). This is Java's behaviour: where Bedrock's hard-coded
drops differ, nothing here knows.

## Mob loot

`mob-loot.json` (read by `mob_loot.rs`) is BDS's own: `data/generate/mob-loot.py` takes each entity's
loot table from the vanilla behaviour packs (their `.brarchive`s, the newest pack version winning), the
adult's in its default state, and every table those name, with old item names renamed (checked against
`ItemRegistry.bin` by a test). Fox and piglin name tables no pack holds; slimes and magma cubes, whose
loot goes by size, have none here. A death is no one's or a player's (`Killer`), with the Looting it
held (`enchant.rs`); a pet's kill counts as no one's. The script also keeps the tables blocks eject,
named in its `BLOCK_TABLES` (trial spawners' and vaults'), rolled by `table_loot`.

## Mob traits

`mob-traits.tsv` (read by `mob_health.rs`): for each entity of the vanilla behaviour packs, which of
these its own components (not its component groups) say, by `data/generate/mob-traits.py`:
`xp=least-most` (`minecraft:experience_reward` of the entity or of a group not named for a baby, the
part after `last_hit_by_player ?`, without equipment), `fire_immune` (`minecraft:fire_immune`), `no_fall` (a `minecraft:damage_sensor` trigger that takes
no damage from `fall` or from all), `breathes_water` and `no_air` (`minecraft:breathable`), `floats`
(`minecraft:behavior.float`), `steadfast` (`minecraft:knockback_resistance` 1) and `resists=` (its
value below 1), `unpushable` (no `minecraft:pushable_by_entity`, nor a `minecraft:pushable` with
`is_pushable`, in the components or any group).
`strolls=` and `panics=` are `minecraft:movement` times the `speed_multiplier` of `random_stroll`
and of `panic`, for an entity with `navigation.walk` (or `navigation.generic` that can walk); a
movement given as a range (a horse's) counts as its middle. `swims=` is `minecraft:underwater_movement`
times `random_swim`'s `speed_multiplier` (with `navigation.generic` that can swim), and `roams=XxY`
that behaviour's `xz_dist` and `y_dist`. `attack=`, `speed=`, `sees=` and `hits=effect:ticks` are the script's own
table of the kinds whose hunting was drilled (`docs/research/mob-attacks.md`): `attack` is
`minecraft:attack` plus what the kind holds (a vindicator's axe 5, a wither skeleton's sword 5),
`speed` is `minecraft:movement` times its attack's `speed_multiplier`, `sees` the `max_dist` of its
`nearest_attackable_target` for players (16 where it names none); `varies=` the top of an
`attack` range BDS draws with fractions (the hoglin's `[3, 9]`).

## Husbandry

`husbandry.tsv` (read by `husbandry.rs`, git-ignored, made by `data/generate/husbandry.py` from the
vanilla behaviour packs; docs/research/husbandry.md): for each entity whose packs raise it, what its
own components or first group holding one say: `breed=`, `mates=`, `breed_tame`, `breed_healthy`
(`minecraft:breedable`); `grows=` ticks and `grow=item:share` (`minecraft:ageable`); `tame=` and
`tame_chance=` (`minecraft:tameable`), with `tame_health=` and `tame_attack=` of its tamed groups;
`temper=least-most` and `temper_feed=item:mod` (`minecraft:tamemount`); `heal=item:amount`; `lays=item:least-most`
ticks (`minecraft:spawn_entity`); `follows=start-stop`, `sits`, `defends`, `leashable`; `tempt=` and
`tempt_speed=`; `walks=` (`minecraft:movement`, a range's middle); and `on_born`, `on_grow`, `on_tame`,
`on_sheared`, `on_regrow` as `-group,+group`, the events followed through their triggers and
unfiltered sequence steps. Old item names are renamed as for mob loot. `raised_look.rs` builds the
SetEntityData of a baby's scale and box (the recorded baby AddEntity's), a sheep's wool, a name tag,
and the owner and lead holder.

## Mob families

`mob-families.tsv` (read by `families.rs`, tested by a selector's `family=`, Smite, Bane of Arthropods and the undead's
potion rules): each entity's
`minecraft:type_family` from the vanilla behaviour packs, by `data/generate/mob-families.py`: its own
components', or, for a kind that has them only in component groups, every group's together. Kinds the
packs give no family (items, falling blocks) are not listed.

## Spawn rules

`spawn-rules.tsv` (read by `spawn_rules.rs`): one row per condition of the vanilla packs'
`spawn_rules/*.json`, the newest pack's file winning, by `data/generate/spawn-rules.py`: kind,
`population_control`, where it spawns (`surface`, `underground`, `underwater`, `lava`), weight, herd
size, then `light=min~max` (`w`: `adjust_for_weather`), `difficulty=`, `height=`, `distance=`,
`density=surface:underground` (`-` for none), `on=` and `not_on=` blocks (old names renamed),
`permute=weight:kind,..`, and the biome filter in prefix form (`&(..)` all of, `|(..)` any of, `!|(..)`
none of, `!tag` not, `snow` for `is_snow_covered`). Left out: conditions that need patrols or villages
(`pillager_patrol`), and the phantom, guardian and slime, whose spawning BDS's own code gates. Herd
events (babies, variants, riders) are not kept. `despawns.tsv`: each entity's own `minecraft:despawn`
`despawn_from_distance` as `near-far` (32 and 128 where it names none). Biome tags are read from
`BiomeDefinitionList.bin` by the ids chunks store (`spawn_rules/biomes.rs`).

## Trades

`trades.json` (read by `trades.rs`) is BDS's own: `data/generate/trades.py` takes the vanilla packs'
`trading/economy_trades/*.json` (the newest pack's file winning; some ship compiled, `MCB`, which the
script reads: format in sapling's `docs/research/villagers.md`), with old item names and data values
renamed (checked against `ItemRegistry.bin` by a test), `set_potion` turned into the item's data value
and biome `filters` into the villager types (`MarkVariant`) they let through. `trades::roll` draws a
tier's offers: `num_to_select` of each group by weight, an alternative of each item, its count, and its
functions (`enchant_with_levels` and `enchant_book_for_trading` through `enchant/table.rs`,
`random_aux_value`, `random_dye`); a trade with an `exploration_map` is never offered.

## Liquid reactions

`liquid-reactions.tsv` (read by `liquid_reaction.rs`): for each of 736 vanilla blocks, what water
does to it (Broken, Popped, Blocking, Noreaction) and whether it keeps a water source beside itself.
Read statically off BDS's block registration, not out of a pack or the Java jar; how, and which rows
are less certain: sapling's `docs/research/waterlogging.md`. A block not listed is Blocking and keeps
nothing, BDS's default.

## Blast resistance

`block-resistance.tsv` (read by `block_resistance.rs`): the explosion resistance of each of BDS's 1477
blocks, read out of a running BDS's memory (`data/generate/block-resistance.py`, which says how), not
the Java game's table. Every state of a block had the same value.

## Map colours

`map-colours.tsv` (read by `map_colours.rs`): each block type's colour on a map, read off maps BDS drew
of every block's default state laid in a grid (`tools/map-colours`, captures
`research/captures/bds-mapgrid*`), by `data/generate/map-colours.py`: the block's
`minecraft:map_color` component where it has one (with its tint), else the full-brightness pixel of a
cell raised a block, else the one palette colour whose normal shade matches, else a colour between whole
bytes (BDS keeps some as floats) fitted to both shades; `-` for blocks maps look through. Only default
states were drawn: a block whose colour changes with its state (logs by axis, beds) has its default's.
`map-tints.tsv` (`data/generate/map-tints.py`): each biome's grass, foliage and water tint, read off a
grass block, oak leaves and water set where `/locate biome` led (`bds-maptints.cells`); biomes today's
worldgen never makes take those of a found biome of the same climate. BDS blends tints between
neighbouring biomes, so a value is the one at the probed spot.

`game_rules`, `no_player_spawn` and `world_spawn` are built, not recorded: the rules are StartGame's, the
spawn packets have the shape `bds_entity_probe` (join) and `bds_mob_recorder` (`/setworldspawn`) print.

## Open

- This is Mojang's data, taken from a server's output rather than a licensed dataset, so it stays out of
  git. DESIGN.md "Open items" tracks replacing it with a source whose licence is clear.
- `StartGame.block_properties` (26.50's data-driven vanilla blocks) and `block_pallette_checksum` are kept
  as recorded. They match acacia-world's block palette for 26.5x and must be re-recorded with it.
- A protocol bump needs a new recording: the templates decode with the current acacia-proto only.
