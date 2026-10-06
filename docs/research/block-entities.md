# Block entities of item-placed blocks (2026-10-06)

Signs, hanging signs, skulls, banners, flower pots, cauldrons, brewing stands, campfires: what the item places, the
block entity BDS keeps, what breaking leaves, and the brewing stand's screen. Conventions as block-rules.md.

Tags: **BDS** = bdsre on `acacia/.testserver/bds-molang/bedrock_server.exe` (VA of the function; no labels exist for
these, the role is read from the code). **REG** = sapling-data `ItemRegistry.bin` / `CreativeContent.bin` (decoded:
network id, damage, block hash) and acacia-world `blocks.bin` (block names, identical to Geyser's
`bedrock/block_palette.26_50.nbt`, whose states are listed). **GEY** = Geyser-Standalone.jar (`mappings/items.json`,
`mappings/blocks.nbt`, translator classes via javap). **JAVA** = server-26.3.jar via javap. **TSV** = sapling-data
`block-loot.tsv` read through `loot.rs`.

NBT putters seen in BDS saves (by what their loads read back): `14284e950` Byte, `14284f590` Byte (bool),
`14284ea60` Short, `14284eb70` Int, `14284ec80` Long, `14284ed90` Float, `14284eeb0` String, `14284f3a0`/`14284e370`
Compound. Every save starts with the base `14486c2f0` (`id`, `x`, `y`, `z`, `isMovable`; sapling's
`new_block_entity` writes no `isMovable`, GEY `FlowerPotBlock.createTag` writes `isMovable:1b`).

## 1. Items and the blocks they place

Block entity ids: BDS id table `141f1d550` (`Sign`, `Skull`, `FlowerPot`, `BrewingStand`, `Cauldron`, `Banner`,
`Campfire`, `HangingSign`; no separate soul-campfire id).

| Item (REG) | Block hash in stack | Places (REG block names, properties) | Choice by clicked face | Evidence |
|---|---|---|---|---|
| `oak_sign` (361) | 0 | Up: `standing_sign` `ground_sign_direction` 0-15; side: `wall_sign` `facing_direction` | Up = standing, rotation = block-rules §1 rule. Side 2-5 = wall, `facing_direction` = clicked face. Down: refuse (JAVA `StandingAndWallBlockItem` tries a wall state by look; block-rules §4 says refuse) | BDS SignItem use `142efa040` (block from vt+0x418 by face); GEY `oak_sign` -> `standing_sign` |
| `spruce/birch/jungle/acacia/mangrove/cherry/bamboo/pale_oak/poplar/crimson/warped_sign` | 0 | `<wood>_standing_sign`, `<wood>_wall_sign` | same | REG |
| `dark_oak_sign` (590) | 0 | `darkoak_standing_sign`, `darkoak_wall_sign` (no underscore) | same | REG, GEY |
| `<wood>_hanging_sign` (oak -500 ... poplar -1130), 13 woods incl. `dark_oak_` | 0 | one block `<wood>_hanging_sign`: `hanging` 0/1, `attached_bit`, `ground_sign_direction`, `facing_direction` | Down face: `hanging=1`; `attached_bit=1` when the block above has no full bottom face or the player sneaks; attached: `ground_sign_direction` = 16-step rotation of yaw+180; not attached: `facing_direction` = cardinal of yaw (2 N, 3 S, 4 W, 5 E), rotation snapped to it. Side face: `hanging=0`, `facing_direction` = opposite of the nearest horizontal look direction not on the clicked face's axis (the board runs along the wall). Up face: refuse | JAVA `HangingSignItem` (attachment UP), `CeilingHangingSignBlock`, `WallHangingSignBlock.getStateForPlacement`; GEY `blocks.nbt` (Java ceiling sign -> `hanging=1`, `attached_bit` = Java `attached`, `ground_sign_direction` = Java rotation) |
| `skeleton_skull` (144), `wither_skeleton_skull` (-965), `zombie_head` (-966), `player_head` (-967), `creeper_head` (-968), `dragon_head` (-969), `piglin_head` (-970) | **nonzero** (already block items) | block of the same name, `facing_direction` 0-5 (blocks are split per kind; `minecraft:skull` 718 is a leftover item, not in creative) | `facing_direction` = clicked face; Up = 1 and the rotation goes in the entity | BDS skull use `142efa9c0`: state `facing_direction` = face; REG |
| `banner` (577), damage = colour | 0 | Up: `standing_banner` `ground_sign_direction`; side: `wall_banner` `facing_direction` | as signs | REG; creative stacks carry tag `{Type:0i}`; GEY `white_banner` -> `banner` damage 15, `red_banner` 1, `black_banner` 0 |
| `flower_pot` (525) | 0 | `flower_pot` `update_bit=0` | any face, needs a block under (block-rules §3) | REG; GEY maps every Java `potted_*` to `flower_pot[update_bit=0]` |
| `cauldron` (436) | 0 | `cauldron` `fill_level=0`, `cauldron_liquid=water` (also `lava`, `powder_snow`; no `lava_cauldron` block in this palette) | any | REG |
| `brewing_stand` (435) | 0 | `brewing_stand` `brewing_stand_slot_a/b/c_bit` = bottle in slot 1/2/3 | any | REG; GEY `BrewingInventoryTranslator` (`HAS_BOTTLE_0..2`) |
| `campfire` (601), `soul_campfire` (637) | 0 | same name, `minecraft:cardinal_direction`, `extinguished=0` | `cardinal_direction` = the look direction (not its opposite as furnaces): `["south","west","north","east"][quarter]` | JAVA `CampfireBlock.getStateForPlacement` (`getHorizontalDirection()`); GEY Java `facing=north` -> `cardinal_direction=north` |

`items.rs` can map names: `*_sign` -> standing/wall by face (`oak` -> bare names, `dark_oak` -> `darkoak_`),
`*_hanging_sign`/`flower_pot`/`cauldron`/`brewing_stand`/`campfire`/`soul_campfire` -> same name,
`banner` -> `standing_banner`/`wall_banner`. Skulls need no mapping, only the turn and the entity.

## 2. Block entity written on placement

| Block | `id` | Fields (type, value on placement) | Evidence |
|---|---|---|---|
| sign, hanging sign | `Sign` / `HangingSign` | `FrontText` Compound, `BackText` Compound, `IsWaxed` Byte 0 | BDS save `1448831f0` (vtable `14a8420f0`) |
| per text side | | `IgnoreLighting` Byte 0, `HideGlowOutline` Byte 0, `SignTextColor` Int -16777216 (ARGB, opaque black), `PersistFormatting` Byte 1, `TextOwner` String "", `Text` String "" (also `FilteredText` String when a filtered copy exists) | BDS side save `144883590`; defaults from ctor `144882170` (colour 0,0,0,1.0; persist 1); GEY `SignBlockEntityTranslator` default colour -16777216 |
| skull (all kinds) | `Skull` | `SkullType` Byte, `Rotation` Float, `DoingAnimation` Byte 0, `MouthTickCount` Int 0 | BDS save `144887cd0` (vtable `14a842380`), ctor `1487ff9c0` |
| banner | `Banner` | `Base` Int = item damage (0 black ... 15 white), `Type` Int 0 (1 = ominous), `Patterns` List of `{Pattern: String, Color: Int}` only when the item has patterns | BDS save `141f0c460`; GEY `BannerBlockEntityTranslator` (`Base` = 15 - Java dye id) |
| flower pot | `FlowerPot` | `PlantBlock` Compound (the plant's block state: `name`, `states`, `version`) only when it holds a plant; empty on placement | BDS save `1457f25c0`, load `1457f2650` (also reads legacy `item`/`mData`); GEY `FlowerPotBlock.createTag` |
| cauldron | `Cauldron` | `Items` List (empty), `PotionId` Short, `PotionType` Short, `CustomColor` Int only when dyed | BDS save `141f28440` (10 legacy slots). Defaults -1/-1 are other servers' (PM/DF), ctor not read |
| brewing stand | `BrewingStand` | `FuelTotal` Short 0, `FuelAmount` Short 0, `CookTime` Short 0, `Items` List of stacks with `Slot` Byte 0-4, non-empty slots only | BDS save `141f21ff0`, load `141f21d00` |
| campfire, soul campfire | `Campfire` | `ItemTime1`..`ItemTime4` Int 0 (always), `Item1`..`Item4` Compound (only when that slot holds food) | BDS save `141f27090`, load `141f26bb0`; GEY `CampfireBlockEntityTranslator` (Java slot n -> `Item{n+1}`) |

Skull details (BDS `142efa9c0`): on an Up click `Rotation = floor(yRot * 16 / 360 + 0.5) * 22.5` (player yaw, not
wrapped, no +180); any other face leaves 0. `facing_direction=1` + `Rotation` = floor skull, 2-5 = wall. A fresh actor
has `SkullType` = -1 (ctor `1487ff9c0` stores 0xffffffff at +0xfc) and nothing on the placement path sets it: BDS saves
byte -1; the client draws the kind from the block name. Writing the kind (0 skeleton, 1 wither, 2 zombie, 3 player,
4 creeper, 5 dragon, 6 piglin) is the legacy meaning; either is harmless. Load accepts legacy `Rot` byte x 22.5 and
`MouthMoving`. Down face: not checked (the item asks the block's may-place, vt+0x210).

### Sign editing

| Rule | Evidence |
|---|---|
| After a player places a sign whose item has no `FrontText`/`BackText` in its tag, BDS calls the player's open-sign (vt+0x668) with front = true: the placer gets `OpenSign { position, is_front: true }` (acacia-proto `packets/part_06.rs:2736`). | BDS `142efa040` |
| The update packet BDS sends for a sign is its save plus `LockedForEditingBy` Long (the editor's actor unique id; invalid id when nobody edits). | BDS getUpdatePacket `144885380` |
| The client answers with `BlockActorData` (position + the sign NBT). Server side (`144885760`, vt slot 19): the editor is looked up by the stored lock id; no editor -> the packet is ignored. For `FrontText` and `BackText`, each present, only `Text` is read (also legacy `Text1`..`Text4` joined by newlines; a `{...}` text is parsed as rich-text JSON); colour, glow, lighting, formatting and owner from the client are ignored. A side whose text changed gets the new text and `TextOwner` = the editor's XUID. Then the lock is cleared and the block entity marked changed (re-sent to viewers). | BDS `144885760`, side load `144884720` (flag 0 = client) |
| No length or line-count limit in the sign code (none in `144884720`/`1448867c0`); a cap, if any, is in the packet handler, not checked. Java caps 4 lines, 90 px wide (GEY `signWidthMax` 90). | BDS; GEY |
| `IsWaxed=1` signs are not editable (vanilla behaviour; the waxed check is not decompiled). | - |

## 3. What breaking leaves

`loot.rs` resolves `java_block(name)`; a block with no table is `Itself`, which tries the **Bedrock block name first**
as an item, then the Java id.

| Block | Should leave (Bedrock items) | sapling today (TSV via loot.rs) | Fix |
|---|---|---|---|
| `standing_sign`, `wall_sign` | `oak_sign` | WRONG: renamed to `oak_sign` (no rows -> Itself), but `minecraft:standing_sign` (63) / `wall_sign` (68) are items in REG, so it drops those | Itself must try the Java id first, or map explicitly |
| `<wood>_standing_sign`, `<wood>_wall_sign`, `darkoak_*` | `<wood>_sign`, `dark_oak_sign` | WRONG: not renamed; REG lists `spruce_standing_sign` etc. as items, so they drop themselves | rename `X_standing_sign` -> `X_sign`, `X_wall_sign` -> `X_wall_sign` (TSV row: `oak_sign` item via `loot_table_of`), `darkoak` -> `dark_oak` |
| `<wood>_hanging_sign` (one Bedrock block for ceiling and wall) | itself | CONFIRMED (Itself, item exists) | - |
| skulls/heads (7) | itself | CONFIRMED (TSV rows; `player_head` profile not copied) | copy the skin profile later |
| `standing_banner`, `wall_banner` | `banner`, damage = `Base`, tag keeps `Patterns`/`Type` | WRONG: Itself -> `minecraft:standing_banner` (item 176 in REG) | name `banner`; damage from entity `Base` (sapling `Item` has `damage`, no tag: patterns lost) |
| `flower_pot` | `flower_pot` + the plant (item of `PlantBlock.name`) | pot CONFIRMED; plant MISSING (no state says it) | read `PlantBlock`, add its item (TSV `potted_<plant>` rows give the Java names) |
| `cauldron` (any fill) | `cauldron`; contents lost | CONFIRMED | - |
| `brewing_stand` | itself + every stack in `Items` | CONFIRMED for itself (TSV row) | spill contents as containers do |
| `campfire` | 2 `charcoal`; Silk Touch: itself | CONFIRMED vs Java (TSV); Bedrock count not decompiled | - |
| `soul_campfire` | 1 `soul_soil`; Silk Touch: itself | CONFIRMED vs Java (TSV) | - |
| campfire food | Items 1-4 drop | MISSING | spill `Item1`..`Item4` |

## 4. Brewing stand screen

| Rule | Value | Evidence |
|---|---|---|
| Window type | `WindowType::BrewingStand` (4) | acacia-proto `types/part_02.rs:1318`; GEY `ContainerType.BREWING_STAND` |
| Slots (block entity `Items` `Slot` = container slot) | 0 ingredient, 1-3 bottles, 4 fuel (blaze powder) | GEY `BrewingInventoryTranslator.javaSlotToBedrock` (Java 3 -> 0, 0-2 -> 1-3, 4 -> 4); BDS save loops 5 slots |
| ItemStackRequest container names | `ContainerSlotType::BrewingInput` (9) slot 0, `BrewingResult` (10) slots 1-3 (the request's slot is the container slot), `BrewingFuel` (11) slot 4 | acacia-proto `types/part_02.rs:1470`; GEY `bedrockSlotToJava` |
| `ContainerSetData` properties | 0 brew time (`CookTime`, 400 at start in Java), 1 fuel amount (`FuelAmount`), 2 fuel total (`FuelTotal`, 20 per blaze powder) | GEY `updateProperty` (Java 0 -> 0, 1 -> 1) and `openInventory` (property 2 = 20) |
| Block state | `brewing_stand_slot_{a,b,c}_bit` = slot 1/2/3 holds a bottle | REG, GEY |

`containers.rs` `kind()` would gain `"minecraft:brewing_stand" => Kind { entity: "BrewingStand", slots: 5,
window: WindowType::BrewingStand, has_lid: false }`; `requests.rs` maps the three slot types to `Place::Container(slot)`
(input 0, result `slot`, fuel 4). Brewing itself out of scope.

## 5. Interactions worth having (brief)

| Click | Effect | Evidence |
|---|---|---|
| Empty `flower_pot` with a pottable plant (Java `potted_*` list: saplings, flowers, mushrooms, fern, dead bush, cactus, bamboo, azaleas, roots, fungi, eyeblossoms, mangrove propagule) | Block state unchanged (`update_bit=0`); entity gets `PlantBlock` = plant block NBT; survival takes one; resend the block entity | GEY `FlowerPotBlock`/`blocks.nbt`; JAVA `FlowerPotBlock` |
| Filled pot, empty hand | Plant back to the hand, `PlantBlock` removed | JAVA `FlowerPotBlock.useItemOn` |
| Lit campfire with raw food | First empty `Item1..4` slot gets one; `ItemTime` counts up, done at 600 ticks (Java `cookingTime`), the cooked item pops out | BDS fields; JAVA campfire recipes |
| Cauldron + water bucket | `fill_level=6`, `cauldron_liquid=water` (buckets needed; out of scope) | REG |

## Open checks (one bot session on BDS)

Placed skull's saved `SkullType` (-1 or kind); cauldron `PotionId`/`PotionType` defaults; sign text cap in the
`BlockActorData` handler; hanging-sign states for a side click; Bedrock campfire charcoal count.
