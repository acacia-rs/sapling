# Shulker boxes and ender chests in BDS (2026-10-06, current BDS build `6ab54bee`, protocol 2193)

Tags as in `containers-crafting.md`. **BDSRE** = the current exe (`acacia/.testserver/bds-molang/bedrock_server.exe`,
the same build as every other test server on this machine). The 1.21.3.01 pdb build is **not on disk any more**
(`bdsre/builds/` is gone), so nothing here is BDSRE-old and no address has a real name: names below are mine,
from vtable slot order (LeviLamina headers) and behaviour. Read with
`py -3 bdsre/bdsre.py c <va>` (`BDS_EXE` set to that exe). Nothing was tried on a live server.

## What sapling needs (summary)

1. Breaking a shulker box drops one box item whose user tag has a top-level `Items` list (compounds with
   `Slot`); placing such an item loads those `Items` into the new block entity. Other keys may ride along.
2. No shulker box (any colour, or undyed) may go into a shulker box: refuse the request.
3. Ender chest: one 27-slot list per player, `EnderChestInventory` in the player record (sapling already
   stores it, `sapling-leveldb/src/player.rs`). The window slot type is `container` 7. The block entity
   keeps no items of its own; its lid and sounds come from the block.
4. Sounds: shulker ShulkerBoxOpen 69 / ShulkerBoxClosed 70, ender chest EnderChestOpen 71 / Closed 72;
   lid `BlockEvent` 1/1 and 1/0 for both, as for a chest.

## Shulker box

**Names** (BDSRE strings in 1425f41e0; DATA `acacia-world/data/blocks.bin`, `sapling-data/data/ItemRegistry.bin`).
Blocks: `minecraft:undyed_shulker_box` and `minecraft:<colour>_shulker_box` for the 16 colours
(white orange magenta light_blue yellow lime pink gray light_gray cyan purple blue brown green red black).
No `minecraft:shulker_box` block. The item registry has those 17 **plus** `minecraft:shulker_box` (legacy
id 218, registered at 1419e3217; not a block, not in BDS's shulker list below). sapling's loot table maps
`undyed_shulker_box` to Java `shulker_box` (`block_loot.rs:46`), and the TSV rows say `copy_components`
(Java), which sapling must replace with the `Items` copy.

**Block entity** `ShulkerBox` (BlockActorType 25). Constructor 144880da0 calls the chest block actor's
(141f0f1a0, type 0x19, renderer 13); vtable 14a841d20 (BlockActor part), Container part 14a841f50.
- `save` 144882000 (BDSRE): `facing` **byte** (field +0x378), then the chest's save (141f313d0: base
  `id x y z`, `CustomName`/`FilteredCustomName` when named (14486c2f0), `LootTable`/`LootTableSeed` when set
  (144878540); pair keys never apply), then `saveItems` (141f31000), then `Findable` byte.
- `saveItems` 141f31000 (BDSRE, shared by every chest): loops 27 slots, writes each non-empty stack as the
  item compound plus `Slot` byte, and **always** writes `Items`, empty list included.
- `load` 144881f70: the chest's load, then `facing` read as a byte (through the load helper).
- The block has **no facing state**: direction is only the block-entity byte (DATA blocks.bin has no state
  list for it; mem: Bedrock `Facing` 0 down 1 up 2 north 3 south 4 west 5 east).

**Breaking** — `ShulkerBoxBlock` vtable 14a94e2e0 (ctor 148b54250, `tile.shulkerBox`).
- `playerWillDestroy` 148b54c10 (slot 76, BDSRE): server side, if the player's game type is Creative (or
  Default with a creative world default) it computes the block's drops (141a23c30) and spawns them (141a241c0)
  **only if the first drop's user tag has an `Items` list with at least one entry**. So in creative an empty
  box drops nothing, a filled one drops itself with its items. Then the base `playerWillDestroy` (141a23960).
- Survival: drops come from the block's drop strategy (`BlockType+0x2b0`, invoked in 141a23c30), set by
  `VanillaBlockDrops::configureBlockDrops`; **the shulker strategy was not found**. What is known: its item
  carries `Items` at the top of its user tag (the creative check reads exactly that), and the chest-family
  `saveItemInstanceData` (141f31670, slot 3 of the shulker vtable) is "full `save()` minus `pairx`/`pairz`"
  (BDSRE), which is the likely source (inf). If so the tag is `{id:"ShulkerBox", x, y, z, isMovable?,
  facing:b, Findable:b, Items:[...], CustomName?}` and an emptied box still drops with `Items: []` (inf; the
  creative test's "at least one entry" only makes sense if empty lists occur).
- Contents never spill: chest-family `onRemoved` (141f32880) only unpairs and notifies, no `dropContents`.
- Tool / silk touch: no tool check found; Bedrock drops the box with any tool or by hand (mem).

**Placing** — item user tag to block entity, 141a87480 (BDSRE; called from the block-item placement paths
142a80e90/142a817f0/142a83310 and from dispensing):
- Only when the item has a user tag. Builds `tag = blockActor.save()`, merges the item's user tag over it,
  then the entries of an optional `BlockEntityTag` compound; drops `RepairCost`; when the item has a custom
  hover name, sets `CustomName` (and `FilteredCustomName`) from it. Then `load`, `loadBlockData`,
  `onCustomTagLoadDone`, `setChanged`. `BlockActor::load` (14486c010/141f1d060) reads no `x y z`, so stale
  coordinates in the item tag are harmless.
- So for sapling: new block entity = its own `id x y z` + the item's `Items` (+ `CustomName` from the item's
  display name). sapling's `with_container_items` fits.
- Dispenser (`ShulkerBoxBlock::dispense` 148b544d0): after placing, `loadItems(userTag)` (slot 27) and
  `facing = table[face]` with table 14a682118 = `[1,0,3,2,5,4]` (the opposite face).
- **Player placement's `facing` writer was not found** (no byte write to +0x378 outside dispense/load). mem:
  facing = the face clicked (box opens away from the surface).

**Slot rule** (BDSRE): one list of 17 item names, global 14cce30b8..c0, filled at 142619914 from 14a7a6cc0:
the 16 colours (white first ... black) and undyed; `minecraft:shulker_box` is not in it.
- `ShulkerBoxBlockActor::canPushInItem` 1448820e0 (hoppers/droppers): false for any item in the list.
- `ShulkerBoxContainerValidation::isItemAllowedInSlot` 142f49090 (item stack requests): false for the same list.
- Hopper transport also refuses them into a type-25 block actor (14903add0/1490397b0).
- Cauldron washing a dyed box gives an undyed box with the **same user tag** (141a435d4).
- Response status: the exe's `ItemStackNetResult` names (147736a40) are listed in the protocol order
  (`Success, Error, InvalidRequestActionType, ...`); which one a refused slot gives was not traced. An honest
  client never sends it (the client runs the same validation, inf); answer `Error` (1).

**Open/close** (BDSRE): `startOpen` 144880fd0 = chest `startOpen` (141f0f4d0, the `BlockEvent` 1/1 lid
logic of containers-crafting A3) + a redstone/comparator update; `stopOpen` 1448810f0 = chest `stopOpen`
(141f323a0) + the same update. Sounds: `getOpenSound` 1422db780 returns 69, `getCloseSound` 144881270 returns
70 (slots 33/34). Window: `ContainerOpen` type Container 0 (mem, as A1); slot type `shulker` 30
(`ShulkerBoxContainer` in the exe's ContainerEnumName table 1476ffc70; DATA `ContainerSlotType::Shulker`).

## Ender chest

**Player storage** (BDSRE): player `readAdditionalSaveData` 1402af220 reads `EnderChestInventory` only if it
is a List (tag type 9) and hands it to the player's ender container (player+0x810, vfunc +0x190 = load);
`addAdditionalSaveData` 1402b2d30 always writes it (vfunc +0x188 = save, the container's list). Layout: list
of item compounds with `Slot` byte (mem; same as `Inventory`); 27 slots (sapling `ENDER`, mem).

**Block entity** `EnderChest` (BlockActorType 23), ctor 1457f24b0 → chest ctor (0x17, renderer 12, chest
kind 2 + ender flag), vtable 14a878bf0. Every slot is the chest's except `openBy`. It saves like a chest
(141f31350 → `Items` always written, here always empty, + `Findable`): BDSRE for the code path, inf that
its own container stays empty.
- `openBy` 1457f25a0: sets the player's ender container's active chest (`*(player+0x810)+0x1b8 = this`, the
  inlined `EnderChestContainer::setActiveChest`), fires a script event, then opens a screen on the block's
  Container part like a chest. The screen's items are the player's ender container (inf: the
  `EnderChestContainer` header overrides `startOpen`/`stopOpen`, which forward to the active chest).
- Lid: chest `startOpen`/`stopOpen` on the **block** (viewer set on the block, so players sharing one ender
  chest share its lid): `BlockEvent` 1/1 on 0→1 viewers, 1/0 on →0 (A3 logic; inf for ender).
- Sounds: chest `getOpenSound` 141f359d0 returns `67 + 4*(kind==2 && flag)` → 71 for an ender chest,
  `getCloseSound` 141f359f0 → 72 (BDSRE).
- Hoppers: `canPushInItem`/`canPullOutItem` overridden (header; folded to false, mem).
- Slot type in requests: `container` 7. The exe's ContainerEnumName table has no ender chest entry, only
  `LevelEntityContainer` (BDSRE 1476ffc70; DATA: acacia-proto `ContainerSlotType` has none either).
- Packets on open: as a chest (A1/A2): `ContainerOpen {window id, Container 0, block pos, -1}` then
  `InventoryContent` for that window id with the 27 ender items (inf from A2; not traced for ender).
- Breaking: obsidian x8, or itself with silk touch (sapling loot TSV, Java); no contents drop.

## Not settled

- The survival shulker drop strategy (exact item tag keys; whether an empty box drops with `Items: []` or
  no tag, which decides whether it stacks with new boxes). Recording a BDS break would settle it.
- `facing` on player placement (clicked face vs opposite; the dispense table is the opposite face).
- Whether the drop copies `CustomName` into the item's display name.
- `ItemStackNetResult` value BDS returns for a shulker box put into a shulker box.
- Ender chest `InventoryContent` container field and that `EnderChestContainer::startOpen` drives the lid.
