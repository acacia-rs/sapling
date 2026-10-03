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
| `ItemRegistry.bin`, `BiomeDefinitionList.bin`, `AvailableEntityIdentifiers.bin`, `CreativeContent.bin` | `registry_packets()`, byte for byte | nothing |
| `UpdateAttributes.bin` (the last one before spawn: the full list) | `update_attributes(id)` | runtime id |
| `UpdateAbilities.bin` | `update_abilities(id)` | unique id |
| `SetEntityData.bin` (the player's own) | `entity_data(id, name)` | runtime id, nametag |

BDS sends more before spawn (PlayerList, CraftingData, AvailableCommands, VoxelShapes, FeatureRegistry,
JigsawStructureData, camera presets, inventory, game rules). sapling does not replay those: they are world
state or features later steps own. `--list` shows where they sit.

## Open

- This is Mojang's data, taken from a server's output rather than a licensed dataset. DESIGN.md "Open items"
  tracks replacing it with a source whose licence is clear.
- `StartGame.block_properties` (26.50's data-driven vanilla blocks) and `block_pallette_checksum` are kept
  as recorded. They match acacia-world's block palette for 26.5x and must be re-recorded with it.
- A protocol bump needs a new recording: the templates decode with the current acacia-proto only.
