# Custom content: items, blocks and the generated pack

Plugins add items and blocks through `Plugin::content` (sapling-api `content.rs`,
`custom_block.rs`); the server registers them once, before any shard is built
(`sapling/src/server/content.rs`), and builds the resource pack they need
(`sapling/src/packs/generated.rs`). Layouts follow dragonfly (MIT, THIRD-PARTY.md) and BDS's
recorded packets.

## Items

| Rule | Source |
|---|---|
| Custom items take network ids from the highest vanilla id + 1 upward; vanilla ids never move. | dragonfly `world/item.go` (`RegisterItem` appends), recorded ItemRegistry.bin |
| A custom item's ItemRegistry entry is `component_based` true, version DataDriven (1). | ItemRegistry.bin (`minecraft:wind_charge` entry), dragonfly `server.go` `itemEntries` |
| Components: `item_properties` holds `allow_off_hand`, `creative_category` (int), `creative_group`, `max_stack_size` (int), `minecraft:icon{textures{default}}`, `hidden_in_commands`; each component (`minecraft:food`, `durability{damage_chance{min 0,max 100},max_durability}`, `hand_equipped`, `max_stack_size`, `cooldown{category,duration,type}`, `damage`, `display_name`, `wearable`, `armor`) is its own compound. | ItemRegistry.bin (apple, wind_charge, iron_spear) decoded field by field; dragonfly `internal/iteminternal/components.go` |
| `hidden_in_commands` is the byte 2 in BDS's own data-driven entries. | ItemRegistry.bin |
| A custom food restores nutrition and saturation = nutrition x modifier x 2, the vanilla formula (pumpkin pie: 8, 9.6). | sapling-data `custom_items.rs` test against the vanilla food table |
| Creative inventory: an item joins the named group its plugin gives, else the last unnamed group of its category (24, 51, 80, 123); custom entry ids follow the recorded last (1980). | CreativeContent.bin |
| Custom items are saved by name, as vanilla ones are. | shard test `custom_items` (leave and rejoin) |

## Blocks

| Rule | Source |
|---|---|
| sapling sends hashed block ids; the client hashes `{name, states}` (FNV-1a 32 over little-endian NBT) for custom states too, so custom runtime ids are server-internal and follow `BlockRegistry::vanilla().len()`. No vanilla id moves. | dragonfly `world/network_block_hash.go`; sapling-leveldb `custom_states.rs` test (stone's hash made this way equals the registry's) |
| States are every combination of the custom properties, the first property changing slowest; booleans are bytes, numbers ints. | dragonfly `world/block_state.go`, `internal/blockinternal` |
| `vanilla_block_data.block_id` is 10000 + index. | dragonfly `server.go:409` (`10000+int32(i)`) |
| The block's item is numbered `255 - block_id`, `component_based` false, version None (2), as BDS numbers the items of blocks past id 255. | ItemRegistry.bin (vanilla block items) |
| StartGame `block_properties` entry: `components`, `menu_category{category,group,is_hidden_in_commands}`, `molangVersion` 13, `vanilla_block_data{block_id}`, `properties[{name,enum}]`, `permutations[{condition,components}]`. | StartGame.bin (`minecraft:light_gray_concrete_slab`), dragonfly `internal/blockinternal/components.go` |
| Components: geometry, material_instances{mappings,materials{face:{texture,render_method,face_dimming,ambient_occlusion}}}, collision_box{enabled,boxes}, selection_box{enabled,origin,size}, light_emission, light_dampening, friction, destructible_by_mining, map_color, transformation; a block with permutations adds `minecraft:on_player_placing`. | dragonfly `internal/blockinternal/components.go` |
| Custom states save to LevelDB by name and states like vanilla ones and load back by name. | sapling-leveldb `custom_states.rs` test |
| Break time is `destructible_by_mining` seconds by hand (hardness = seconds / 1.5); drops are the plugin's (`BlockDrop`). | sapling choice; shard test `custom_blocks` |
| Server physics reads a custom block with a collision box as stone and one without as air. | sapling choice (`blocks/custom.rs` `stand_in`) |

## The generated pack

| Rule | Source |
|---|---|
| Files: `manifest.json`, `textures/items/*.png` + `textures/item_texture.json` (`atlas.items`), `textures/blocks/*.png` + `textures/terrain_texture.json` (`atlas.terrain`, padding 8, mips 4), `models/blocks/*.geo.json`, `blocks.json` (format `[1,1,0]`), `texts/en_US.lang` (`item.<id>.name`, `tile.<id>.name`) and `texts/languages.json`. | dragonfly `internal/packbuilder` |
| One header uuid for every server; the version is `1.x.y` from the SHA-256 of the sorted files, so it changes exactly when the content does. | sapling choice; dragonfly makes a random uuid each start |
| The pack goes under the server's own packs and is sent with them. | sapling integration test `custom_content` |

## Not drilled

Only a real client can confirm these; none were checked against BDS or a client:

- textures showing on items and blocks, and the lang names
- custom geometry rendering, material instances per face, render methods
- permutation visuals (lit lamp texture or light) and transformation rotations
- armour rendering of a wearable custom item; the cooldown bar
- that a client accepts `hidden_in_commands` 2, `molangVersion` 13 and the creative group placement for custom entries
- the client's own break time and selection box for a custom block
- collision boxes other than a full cube (the server treats them as full: TODO in `blocks/custom.rs`)
- a cooldown is not enforced server-side; eating a custom food was checked only in data
