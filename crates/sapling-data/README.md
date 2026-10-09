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
| `Mobs.bin` (110 kinds, 40 of them also as a baby) | `add_entity(&AddEntityOptions)` | ids, position, rotation, velocity (none), `Variant`, `MarkVariant`, `Color`, name tag, the baby flag, extra flags, health |
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
network id a craft request names: what each cell takes and what a craft makes.

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
loot goes by size, have none here.

## Fire immune

`fire-immune.txt` (read by `mob_health.rs`): the entities whose own components hold
`minecraft:fire_immune` in the vanilla behaviour packs, by `data/generate/fire-immune.py`.

## Liquid reactions

`liquid-reactions.tsv` (read by `liquid_reaction.rs`): for each of 736 vanilla blocks, what water
does to it (Broken, Popped, Blocking, Noreaction) and whether it keeps a water source beside itself.
Read statically off BDS's block registration, not out of a pack or the Java jar; how, and which rows
are less certain: sapling's `docs/research/waterlogging.md`. A block not listed is Blocking and keeps
nothing, BDS's default.

`game_rules`, `no_player_spawn` and `world_spawn` are built, not recorded: the rules are StartGame's, the
spawn packets have the shape `bds_entity_probe` (join) and `bds_mob_recorder` (`/setworldspawn`) print.

## Open

- This is Mojang's data, taken from a server's output rather than a licensed dataset. DESIGN.md "Open items"
  tracks replacing it with a source whose licence is clear.
- `StartGame.block_properties` (26.50's data-driven vanilla blocks) and `block_pallette_checksum` are kept
  as recorded. They match acacia-world's block palette for 26.5x and must be re-recorded with it.
- A protocol bump needs a new recording: the templates decode with the current acacia-proto only.
