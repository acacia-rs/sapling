# BDS containers and crafting: ground truth for sapling (2026-10-05, BDS 1.26.5x, protocol 2193)

Tags: **REC** = RECORDED-FROM-BDS, **DATA**, **BDSRE** (current exe), **BDSRE-old** (the 1.21.3.01 build that
shipped its pdb, real function names; behaviour may have drifted since), **Java**, **mem** (from memory,
project named), **inf** (inferred). Nothing here was tested against a live server in this pass.

- `AS` = `C:/Users/Hexay/acacia/acacia-server`; `DUMP` = `research/sapling-truth/CraftingData-dump.txt`.
- BDSRE: `py -3 C:/Users/Hexay/acacia/bdsre/bdsre.py c <va>`. BDSRE-old: the same with
  `BDS_EXE=C:/Users/Hexay/acacia/bdsre/builds/1.21.3.01/bedrock_server.exe`.
- No chest, barrel, hopper or shulker traffic was ever recorded: the acacia captures cover the crafting table,
  furnace, stonecutter and smithing table only. Sections A-C and G therefore rest on BDSRE and memory.

## sapling gaps ranked by how likely a real client notices

1. **Shift-click crafts that make more than 64 are refused.** `Craft::start` wants `each * times <= 64`
   (`sapling-shard/src/inventory/crafting.rs:26`); 17+ logs into planks fails. BDS keeps the surplus pending
   behind slot 50 (E3).
2. **Stored double chests.** sapling sends the stored compound, so a client that gets `pairlead/pairx/pairz`
   draws one large chest, while sapling opens 27 slots at the clicked half. BDS opens 54 at the lead (B).
3. **Recipe book.** Resolved: sapling keeps and sends each player's book as BDS does (E5).
4. **Barrels do not look open** (no `open_bit` block change), and **no container sounds** at all (A3, C).
5. **Cake, honey block, honey bottle to sugar** are refused: the only three recipes with two outputs (E4).
6. **Inventory screen `ContainerOpen` entity id**: sapling sends the player's id, BDS sends -1 (A1).
7. Furnaces, stonecutters, smithing tables, hoppers, shulker and ender chests do not open (D, F, G, C).
8. Not visible to an honest client: no grid check for `CraftRecipe` (E1), no reach check (A5), the lid's
   `BlockEvent` is not repeated for players who arrive later (A3), a blocked chest still opens (A1).

## A. Chest open and close

1. **Open.** `ChestBlock::use` (BDSRE-old 141bfa780): needs the open-containers ability, then
   `ChestBlockActor::canOpen`, then `openBy`. `_canOpenThis` (BDSRE-old 1420e48e0) refuses when the block
   above is solid and "obstructs chests"; a double chest checks both halves (mem). `openBy` (BDSRE-old
   1420f4de0) opens the screen at the **lead** half when a non-lead half was clicked.
   `ContainerOpen {window_id, window_type Container (0), coordinates = that block, runtime_entity_id -1}`.
   REC for the shape (`AS/docs/research/vanilla-actions-2026-10-02.md` "Cross-cutting rules": -1, 16-157 ms
   after the click). The inventory screen is the same packet with type Inventory (-1), the player's block and
   entity id **-1**: `ServerPlayer::openInventory` (BDSRE-old 140ccddf0) passes 0xffffffffffffffff.
   Window ids: +1 per open, 99 wraps to 1 (`ServerPlayer::_nextContainerCounter`, BDSRE-old 140cc08d0; REC:
   ids ran 2 to 32). sapling already does this.
2. **Contents.** `_setContainerManager` (BDSRE-old 140cc1050) then sends
   `InventoryContentPacket::fromPlayerInventoryId(window id, player)`, the builder that also makes the
   player's own windows. Those were recorded as `container {AnvilInput (0), dynamic None}` with an empty
   `storage_item`, so a chest's content should carry the same two fields (inf). 27 items, or 54.
3. **Lid and sound.** `ChestBlockActor::startOpen` (BDSRE-old 1420fa720): a new viewer is added to a set; when
   the set goes 0 to 1, `BlockSource::blockEvent(pos, 1, 1)`, i.e. `BlockEvent {position, type 1, data 1}`.
   `stopOpen` (BDSRE-old 1420fa980): set empty, then `blockEvent(pos, 1, 0)`. The set lives on the lead half.
   `ChestBlockActor::tick` (BDSRE-old 1420fc690): **while open, the open event is sent again every 80 ticks**;
   the open sound plays on the first tick the lid starts from 0, the close sound when the lid angle falls
   through 0.5, a few ticks after the close (inf: the two calls are virtual and were matched to the sound
   functions by their place in the lid logic; lid speed not read). Sounds are `LevelSoundEvent`:
   ChestOpen 67 / ChestClosed 68, position = block centre (+0.5 each axis; the midpoint of the two halves of a
   double chest), extra data -1, entity type "", baby false, global false (`playOpenSound` BDSRE-old
   1420f5800, `playCloseSound` 1420f5580; ids in `AS/tools/codegen/data/protocol.json:3721`).
   That the server sends the sound rather than leaving it to the client: mem (PocketMine-MP, dragonfly both
   send `BlockEvent` 1/1 and the sound).
4. **Slot changes for viewers.** `LevelContainerManagerModel::broadcastChanges` (BDSRE-old 141a2a320) runs
   every tick per viewer: it compares each slot with that viewer's last-sent copy and calls
   `ServerPlayer::slotChanged` (BDSRE-old 140cd7660), which sends **one `InventorySlot` per changed slot**
   `{window_id = the open window, slot, item}`; only a change in slot count sends `InventoryContent`. In
   protocol 2193 `InventorySlot` also has optional `container` and `storage_item`
   (`protocol.json:9795`): what BDS puts there for a chest is unknown; send them absent or as in A2 (inf).
   The acting player gets them too in the furnace capture ("InventorySlot before each response", REC,
   `vanilla-actions` "Furnace"); for chests unknown. sapling resends the whole window: works, costs bytes.
5. **Walking away.** `LevelContainerManagerModel::isValid(pickRange)` (BDSRE-old 141a2b800): the block entity
   must still exist and `distance²(player position, block centre) <= pickRange²`. The value the caller passes
   was **not recovered**; Java closes at 8 blocks (Java, mem). Closing from the server:
   `ServerPlayer::doDeleteContainerManager` (BDSRE-old 140cc45a0) waits for pending requests
   (`ItemStackNetManagerServer::tryCloseContainerScreen`), then `ContainerClose {window_id, window_type of the
   screen, server true}` (fields: mem, gophertunnel). A client's own close is `{id, window_type None, server
   false}` and BDS echoes it unchanged (REC, same doc "ContainerClose"); PocketMine-MP answers every client
   close, known id or not, since the client waits for it (mem). sapling does both.

## B. Double chests

- **Block entity fields**, written by `ChestBlockActor::_saveClientSideState` (BDSRE 141f313d0) and so sent
  to clients too, via `_getUpdatePacket` (BDSRE 141f32c90): `pairlead` (byte 0/1), `pairx` (int), `pairz`
  (int), on both halves; `forceunpair` (byte) instead when unpairing is forced. `load` (BDSRE 141f30cc0)
  reads the same keys plus `Findable` and `ConvertedFromConsole`; only the lead re-pairs from them.
- **Which chests pair**, `canPairWith` (BDSRE-old 1420ea2f0): the neighbour is a chest block entity, at the
  same y, of the **same block** (chest with chest, trapped with trapped), not paired yet, with the same
  facing state; and it must sit beside, not in front or behind: an x-neighbour needs a north/south facing, a
  z-neighbour east/west (the test is on the raw state value, read as cardinal south 0, west 1, north 2,
  east 3: inf).
- **On placing**, `onPlace` (BDSRE-old 1420f4ad0) tries the two side neighbours in turn through
  `_tryToPairWith` (1420e8910): a cancellable block event, then the old chest's screens are force-closed,
  `new.pairWith(old, lead false)`, `old.pairWith(new, lead true)`: **the chest that was already there
  leads**. No sneak exception is in that path. Java pairs through block state and skips it when sneaking
  (Java: `research/java-truth/decomp/src/net/minecraft/world/level/block/ChestBlock.java:204-232`).
- **Slot order**, `getItem` (BDSRE-old 1420ef940): slots 0-26 are the lead's, 27-53 the other half's. Each
  half keeps its own 27 `Items`.
- **Unpairing**: `tick` (BDSRE-old 1420fc690) drops the pair when the partner's block entity is gone or
  `canPairWith` no longer holds; `onRemoved` unpairs the survivor. After either, send the survivor's
  `BlockEntityData` without the pair keys (inf).
- A trapped chest's signal is the viewer count of both halves, capped at 15 (`ChestBlock::use`).

## C. Barrel, trapped chest, ender chest, shulker box

| Block | Block entity id | ContainerOpen type | Slot type in requests | Specials |
|---|---|---|---|---|
| chest, trapped chest | `Chest` (mem) | Container 0 | `container` 7 | pairs only with its own kind (B) |
| barrel | `Barrel` | Container 0 | `barrel` 58 | `open_bit`; never pairs |
| shulker box | `ShulkerBox` | Container 0 | `shulker` 30 | `facing` byte in NBT |
| ender chest | `EnderChest` | Container 0 | `container` 7 | per-player items |

- Block entity ids: mem. Slot types: `AS/crates/acacia-bot/src/items/slot.rs:139-164` (built against BDS;
  barrel and shulker differ only by the block) and `protocol.json:3573`. Ender chest's: mem (Geyser).
- Barrel: `BarrelBlockActor::startOpen` (BDSRE-old 1420fa560) runs the chest's code (so the `BlockEvent` 1/1
  goes out as well) and, when viewers go 0 to 1, `BarrelBlock::setOpen(true)` (141bb91d0): the block is set
  again with `open_bit` 1 and its facing kept, which reaches clients as `UpdateBlock`. Sounds BarrelOpen 270
  / BarrelClose 271 (`playOpenSound` 1420f56c0).
- Shulker box: `ShulkerBoxBlockActor::save` (BDSRE-old 1422bca20) writes a byte under a 6-letter key
  (`facing`, mem for the name) and then the chest's client state; `startOpen` (1422be700) is the chest's.
  Sounds ShulkerBoxOpen 69 / Closed 70 (1422ba870). Refuses shulker boxes as content
  (`itemAllowedInSlot`); breaking drops the box with its `Items` in the item's tag (mem).
- Ender chest: `EnderChestBlockActor::openBy` (BDSRE-old 1422ba450) first calls
  `EnderChestContainer::setActiveChest` on the **player's** container, then opens like a chest. 27 slots kept
  with the player (`EnderChestInventory` in the player record: mem, PocketMine-MP). The lid and sound
  (EnderChestOpen 71 / Closed 72, 1422ba750) belong to the block; breaking it drops no contents (mem).

## D. Furnace, blast furnace, smoker

- **Screen.** `ContainerOpen` type Furnace 2 / BlastFurnace 27 / Smoker 28, entity -1, then 3 items.
  Request slot types: slot 0 `furnace_ingredient` 25 (`blast_furnace_ingredient` 45, `smoker_ingredient`
  46), slot 1 `furnace_fuel` 24, slot 2 `furnace_output` 26 for all three; only Take, Place, Swap. REC:
  `vanilla-actions` "Furnace" (-33, -35, -37), `AS/docs/research/workstations.md` "Wire facts",
  `acacia-bot/src/items/slot.rs:142-150`.
- **What the capture showed** (REC, same section): `InventorySlot` before each response; the fuel lit 42 ms
  after its Place and a `BlockEntityData` followed; the ingot arrived as `InventorySlot` about 10 s later;
  taking it was `Place 1 FurnaceOutput 2 -> HotbarAndInventory 11`.
- **`ContainerSetData`**, `FurnaceContainerManagerModel::broadcastChanges` (BDSRE-old 1403503e0), every tick
  per viewer, each only when its value differs from the last one sent: property 0 = cook ticks so far, 1 =
  lit ticks left, 2 = lit ticks the current fuel gave, 3 = stored XP. While smelting that is two packets a
  tick (0 and 1). `{window_id, property, value}` (`protocol.json:9826`).
- **Tick**, `FurnaceBlockActor::tick` (BDSRE-old 1422bf090), in this order: if lit, `lit -= 1`; if not lit and
  `canBurn`, `lit = litDuration = getBurnDuration(fuel, interval)` and one fuel is used up (a lava bucket
  leaves a bucket); if lit and `canBurn`, `cook += 1`, and at `cook == interval` the item is smelted and
  `cook = 0`; otherwise `cook = max(cook - 2, 0)`. When lit flips, `FurnaceBlock::setLit` swaps the block
  (`lit_furnace`, `lit_blast_furnace`, `lit_smoker`, facing kept: names mem) and the entity stays.
  While lit a sound plays every 20-99 ticks (random): FurnaceUse 283 / BlastFurnaceUse 279 / SmokerUse 278.
- **Times.** `interval` is 200 ticks for a furnace, 100 for the other two (mem; REC 10 s per ingot).
  `getBurnDuration` (BDSRE-old 1422b2c70) is `interval x factor`, so a fuel smelts the same number of items
  in every kind. An item with a `minecraft:fuel` component gives `interval x duration x 0.1` (min 1); in the
  extracted packs searched only `wooden_spear` has one (DATA: pack `vanilla_1.21.130`,
  `items/wooden_spear.json`), so **the rest is compiled in**. Factors seen in the code: 80, 20, 1.5, 0.5,
  0.25. Usual table (mem, Bedrock wiki):
  lava bucket 100 items, coal block 80, dried kelp block 20, blaze rod 12, coal/charcoal 8, boat 6, logs,
  planks and most wooden blocks 1.5, wooden tools 1, stick, sapling, wool 0.5, bamboo, scaffolding 0.25.
- **Recipes.** In BDS's own list they are `shapeless_recipes` whose `block` is `furnace` (134), `blast_furnace`
  (62), `smoker` (9), `campfire` (9), `soul_campfire` (9): one input, one output, own network id (DATA: `DUMP`
  line 221111 `minecraft:furnace_beef`; counts by script). sapling's `recipes.rs` filters them out by
  `block == "crafting_table"` and can read them the same way. The JSON carries no time or XP (DATA:
  `recipes.brarchive` `recipes/furnace_beef.json`: `tags ["furnace","smoker","campfire"]`, input, output).
- **XP.** The request carries nothing (REC, `workstations.md`). The entity adds each smelt's XP in hundredths
  to `StoredXPInt`; taking the output withdraws it, rounded (`withdrawStoredXPReward` BDSRE-old 1422c2c80).
  Per-item values are compiled in (`Item::getFurnaceXPmultiplier`); Java's table is the fallback (mem: iron
  and copper 0.7, gold, diamond, emerald 1.0, food 0.35, glass, stone, coal ore 0.1, ancient debris 2.0).
- **Block entity**, `FurnaceBlockActor::save` (BDSRE-old 1422bb4b0; key strings present in the current exe):
  `BurnTime` short, `CookTime` short, `BurnDuration` short, `StoredXPInt` int, `Items` list with `Slot`
  byte. Ids `Furnace`, `BlastFurnace`, `Smoker` (mem). Its update packet carries no items.

## E. Crafting validation

1. **BDS checks the grid for `CraftRecipe`.** `CraftHandlerCrafting::_handleCraftAction` (BDSRE 1446df030):
   net id lookup (miss: status 32), a 3x3 recipe outside a workbench is 33, then it reads its own
   `CraftingInput` cells (4 from slot 28, or 9 from 32), records an expected consume of `numCrafts` for every
   non-empty cell, and calls `recipe->matches(grid)`: false is status 35
   `MismatchedRecipeForInputGridItems`. The results come from `recipe->assemble(grid)`.
   `handleConsumedItem` (BDSRE 1446de6a0) then holds each `Consume` to those cells (statuses 20-27); a
   `Consume` with no craft action before it is 7. sapling only totals what was consumed.
2. **`ResultsDeprecated` is ignored for crafting** (BDSRE 148b73310: returns 0 unless the handler trusts the
   client's results). With no craft handler it dereferences null: the recorded anvil/stonecutter crash (REC,
   `AS/docs/research/live-action-tests.md` fixes 3 and 5). sapling ignoring it is right.
3. **Counts.** Results are kept as per-craft count plus a pending `count x (numCrafts - 1)` (BDSRE
   148b736a0). Slot 50 is given at most one max stack, the rest stays pending (BDSRE 148b72ea0). That it is
   topped up as the client empties it, with several Place actions out of slot 50, is inf: only 16 planks in
   one Place were recorded (request -19).
4. **Several results.** Three recipes: `minecraft:cake` (net id 543: cake x1, bucket x3),
   `minecraft:honey_block`, `minecraft:honey_bottle_to_sugar` (DATA: `DUMP` 37844-38066, 87240, 87344).
   Client: craft action, `ResultsDeprecated` listing both, Consumes, then per result `Create{index}` and its
   Take/Place (mem: Geyser issue 3682, quoted in `workstations.md` "Sequences"; **no 1.26 capture**).
   Server, `Create` (BDSRE 148b72ea0): index past the list 10, bad item 11, no output container 13, **slot 50
   still holding something 14**, over the max stack 64; so each result is moved out before the next
   `Create`. A single result needs no `Create` (REC -19, -27).
5. **Unlocking.** BDS sends `UnlockedRecipes` (656 bytes) during join (REC: `sapling-data/README.md:27`, the
   join recording's packet list). sapling's `StartGame.bin` has `recipesUnlock` true, `doLimitedCrafting`
   false (DATA: bytes at offsets 164 and 181). `_handleCraftAction` returns no status for a locked recipe,
   and BDS crafted a locked recipe with `doLimitedCrafting` on (REC: research/captures/bds-limitedcraft.txt;
   the rule paused unlocking instead). What unlocks what: research/captures/bds-recipeunlock*.txt and
   `sapling_data::unlocked_by`; sapling's book: `inventory/recipe_book.rs`.
6. **Statuses.** Numbers are from the decompiles and REC; names are mem (Mojang's `ItemStackNetResult`),
   three of them checked as strings in the exe (`MismatchedRecipeForInputGridItems`,
   `MismatchedCraftingSize`, `InvalidCraftResultIndex`): 7 InvalidCraftRequest, 9 InvalidCraftResult,
   10 InvalidCraftResultIndex, 11 InvalidCraftResultItem, 25 a consumed item fails the expected descriptor
   (REC: a tag in `CraftRecipeAuto`), 27 ConsumedItemNotAllowed (REC: Consume from `Hotbar`), 31/32 recipe
   not found, 33 MismatchedCraftingSize, 35 grid mismatch, 36 EmptyCraftResults, 63 InvalidNumCrafts.

## F. Stonecutter and smithing table

- **Which screens have a craft handler**, `_createCraftHandler_byContainerType` (BDSRE 148b72080), by window
  type: workbench 1 and inventory: crafting; 3 enchant; 15 trade; 24 loom; 26 grindstone; **33 smithing**.
  Furnace 2, anvil 5, **stonecutter 29**, cartography 30 and the rest get none: their craft goes through
  `_screenSimCraft` (BDSRE 1446d6150), where the server works out the result from its own slots and the
  recipe id, and anything but exactly one result is status 9.
- **Stonecutter** (REC: `vanilla-actions` "Grindstone, stonecutter, smithing", window 14): `ContainerOpen`
  type Stonecutter 29. -85 `Place 4 -> StonecutterInput` (slot type 53, UI slot 3). -87 `CraftRecipe {450,
  times 4}`, **no ResultsDeprecated**, `Consume 4` from slot 3, `Place 4` from `CreatedOutput` 50.
  Validate: the id is a recipe with `block "stonecutter"` (354 in `DUMP`, shapeless, one input), the input
  slot holds its ingredient, `times <= ` input count, placed count = output count x times.
- **Smithing** (REC, window 15): type SmithingTable 33; template to UI slot 53 (`smithing_table_template`
  61), material to 52 (`smithing_table_material` 4), base to 51 (`smithing_table_input` 3). -99 `CraftRecipe
  {1567, 1}` + `ResultsDeprecated` (netherite sword) + three Consumes (53, 51, 52) + Place from 50.
  Recipes are `smithing_transform_recipes` (12: template, base, addition, result, tag `smithing_table`) and
  one `smithing_trim_recipes` entry on item tags (net id 1558) (DATA: `DUMP` 564370, 565164). The result keeps
  the base's damage, enchantments and name (mem); BDS takes it from its own handler, not from the request.
- Items left in station slots go back to the inventory on close (mem: PocketMine-MP, dragonfly).

## G. Hopper

`ContainerOpen` type Hopper 8, 5 slots, request slot type `container` 7 (mem: Geyser's hopper translator;
`HopperContainerManagerModel` exists, BDSRE-old 140349180). Block entity id `Hopper`, `Items` with `Slot`,
`TransferCooldown` int (mem).

## Not settled here

- The reach at which BDS closes a screen (A5), and the lid speed behind the close sound's delay (A3).
- `InventoryContent`/`InventorySlot` `container` and `storage_item` for a chest window (A2, A4).
- The multi-result sequence on 1.26 and the order of `ResultsDeprecated` entries (E4).
