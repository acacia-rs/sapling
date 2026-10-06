# BDS furnaces, blast furnaces, smokers: what sapling still lacks (2026-10-06)

Tags as in `containers-crafting.md` (REC, DATA, BDSRE = current exe, BDSRE-old, mem, inf). **Everything here is
BDSRE on the current exe** (`acacia/.testserver/bds-molang/bedrock_server.exe`): the 1.21.3.01 build with its pdb is
not on this machine (`bdsre/builds/` does not exist), so no BDSRE-old. Names below are mine unless LeviLamina
headers give them (LL). Run: `BDS_EXE=<that exe> py -3 C:/Users/Hexay/acacia/bdsre/bdsre.py dis <va>`.
Block/item names were read by matching each `HashedString` global's init immediate (FNV-1 64, not 1a) or its
string; scratch scripts in the session scratchpad, not kept.

## 1. Fuel: how long one piece burns

`FurnaceBlockActor::getBurnDuration(stack, float interval)` **1457f2a30**:
1. Item has a `minecraft:fuel` component (data-driven items, and `CoalItem`): `max(1, interval * duration / 10)`.
2. Otherwise tail-calls `getItemBurnDuration(item, interval)` **1457f2bb0** (rules below, first match wins).
- The tick stores the float with **`cvttss2si` (truncation)** into lit time and lit duration (1457f4a1c).
- `interval` comes from the actor: furnace 200, blast furnace 100, smoker 100 (constructor calls 1487ff5a3,
  148801933, 148801cc3 pass 0xc8 / 0x64 / 0x64). A fuel smelts the same number of items in all three.

`getItemBurnDuration`, with `B` = the item's block type (`Item+0x178`, LL `mBlockType`; null for most items):
| # | Condition | Factor (items smelted) |
|---|---|---|
| a | `B` null | `Item+0x1b0` (LL `mFurnaceBurnIntervalModifier`; 0 = not fuel) |
| b | `B` is one of 48 blocks below ("nether list") | 0 |
| c | `B` has `BlockProperty::Button` (0x100, LL) and material Wood (`Material::mType == 2`) | 0.5 |
| d | `B` is a sapling (oak, spruce, birch, jungle, acacia, dark_oak, cherry, pale_oak, poplar), `mangrove_propagule`, `azalea`, `flowering_azalea`, `deadbush`, `leaf_litter`, `short_dry_grass`, `tall_dry_grass` | 0.5 |
| e | `B` is `ladder`, or material Wood | 1.5 |
| f | `B` is `coal_block` | 80 |
| g | `B` is `dried_kelp_block` | 20 |
| h | `B` is `bamboo` or `scaffolding` | 0.25 |
| i | anything else | `Item+0x1b0` |

Nether list (b), all `minecraft:`: crimson/warped `_planks`, `_stem`, `stripped_*_stem`, `_hyphae`,
`stripped_*_hyphae`, `_slab`, `_double_slab`, `_stairs`, `_fence`, `_fence_gate`, `_door`, `_trapdoor`,
`_pressure_plate`, `_button`, `_standing_sign`, `_wall_sign`, `_fungus`, `_roots`, `_nylium`, `_shelf`, and
`chorus_flower`, `chorus_plant`, `red_mushroom_block`, `brown_mushroom_block`, `mushroom_stem`, `creaking_heart`,
`beehive`, `bee_nest`. So BDS excludes nether wood by **block name, not by tag or flammability**.

Which blocks are material Wood: confirmed from the inlined `Material::getMaterial(2)` in block registration
14849c7e0 (125 calls; names read for most): every species' planks, slabs and double slabs, fences, doors,
trapdoors, pressure plates, buttons, `bamboo_mosaic` (+slab), `fletching_table`, the mushroom blocks. Logs, chests,
bookshelf, crafting/cartography/smithing tables, barrel, lectern, composter, loom, jukebox, note block, daylight
detector, stairs and fence gates were not located (other functions); that they are Wood is mem/inf.

`Item+0x1b0` per item, set in item registration **141fc2880** (`mov dword [item+0x1b0], imm`):
| Item | Factor |
|---|---|
| `lava_bucket` | 100 (the bucket stays: see section 4) |
| `blaze_rod` | 12 |
| every boat, chest boat, `bamboo_raft`, `bamboo_chest_raft` (oak..pale_oak, mangrove, cherry, poplar) | 6 |
| every `*_hanging_sign` except crimson/warped | 4 |
| `fishing_rod`, `banner`, `mangrove_roots` | 1.5 |
| `wooden_sword/shovel/pickaxe/axe/hoe`, `bow`, `crossbow`, `bowl`, `wooden_door`, `oak_sign`, and each species' `_door` / `_sign` item except crimson/warped | 1 |
| `stick`, `short_dry_grass`, `tall_dry_grass` | 0.5 |
| the 16 `*_cushion` items | 0.25 |

- `coal`, `charcoal`: `CoalItem` (registered by 142018440) attaches a fuel component with duration 80 in its
  constructor **147aa0e70** (`mov [comp+0x20], 80.0`): rule 1 gives 8 items. No other code-built fuel component.
- `wooden_spear`: data-driven, `minecraft:fuel {"duration":10.0}` (DATA: `behavior_packs/vanilla_1.21.130/
  __brarchive/items.brarchive`, the only fuel component in the packs) = 1 item.
- **Not fuel in BDS** (no write found; inf): wool, carpets (no 0.335 constant anywhere), crimson/warped door,
  sign and hanging-sign items. Wooden slabs are **1.5, not 0.75** (rule e). Bamboo item 0.25 via its block.
- Ticks for a furnace = `trunc(200 * factor)`: lava 20000, coal block 16000, kelp block 4000, blaze rod 2400,
  coal 1600, boat 1200, hanging sign 800, log/plank 300, tool 200, stick/sapling/button 100, bamboo 50.

## 2. Experience

- **The stored XP never grows.** No code adds to `StoredXPInt` (`FurnaceBlockActor+0x294`): byte scan of .text
  for add/sub on `+0x294` found nothing; its only writers are load, two withdrawals and the reset. A world saved by
  an older version can still carry some.
- **On taking from the output** (ItemStackRequest, container FurnaceResult 26, slot 2): **142f482a0** builds a
  reward of `getXPRewardFromSmeltingItems(taken stack, taken count)` **1457f5ee0**; the reward's apply
  **148b7f020** adds `round_random(stored / 100)`, zeroes the store and calls `ExperienceOrb::spawnOrbs(region,
  player position, total, ...)` **142f9eb90**: **orbs spawn at the player**, server side only.
- `FurnaceContainerManagerModel` slot write **1480523d0**: when slot 2's count drops and the furnace is not
  emptied by a hopper below (`isEmptiedByHopper` **1457f5cf0**: block actor type 15 under it), it also calls
  `withdrawStoredXPReward` **1457f62d0** and discards the result.
- `round_random(x)` (1457f62d0, 1457f5ee0): `n = trunc(x)`; if `x` is not whole and `nextFloat() >= ceil(x) - x`,
  `n + 1`. That is floor plus a chance equal to the fraction (Java's rule).
- XP per item = `count * _getXPRewardMultiplier(output)` **1457f60a0**: the output item's virtual
  `getFurnaceXPmultiplier` (default `Item+0x1b4`), overridden for block outputs: `hardened_clay` 0.35; `sponge`,
  `wet_sponge` 0.15; `glass`, `stone` (item id 1), `stone_bricks`, `mossy/cracked/chiseled_stone_bricks`,
  `smooth_stone`, `sandstone`, `chiseled/cut/smooth_sandstone`, `red_sandstone` and its three, `quartz_block`,
  `chiseled_quartz_block`, `quartz_pillar`, `smooth_quartz`, `deepslate`, `smooth_basalt`,
  `cracked_deepslate_tiles`, `cracked_deepslate_bricks`, `leaf_litter` 0.1.
- `Item+0x1b4` set in 141fc2880: `netherite_scrap` 2; `diamond`, `emerald`, `gold_ingot` 1; `iron_ingot`,
  `copper_ingot` 0.7, and 0.7 on the item after `iron_door` (redstone: inf); cooked porkchop, beef, chicken, cod,
  salmon, mutton, rabbit, `baked_potato` 0.35; `brick` 0.3; all 16 dyes, `ink_sac`, `bone_meal`,
  `cocoa_beans`, `lapis_lazuli`, `quartz` 0.2; `dried_kelp`, `gold_nugget`, `iron_nugget`, `copper_nugget`,
  `netherbrick`, `popped_chorus_fruit`, `resin_brick` 0.1. Anything else 0 (charcoal included).
- **Breaking the furnace** (`onFurnaceBlockRemoved` **1457f6f60**, from 147163d00): unless `mNoDrop`, spawns
  `round_random(output count * multiplier) + round_random(stored / 100)` orbs at the block position (integer
  corner, no +0.5: inf from the int-to-float copies), zeroes the store, then drops the contents.
- Hoppers pulling the output give nobody XP (no reward path; inf).

## 3. Crackling sound: the server sends it

`FurnaceBlockActor::tick` **1457f4630** (vtable of the tick component; this = actor+0x40), while lit and **not
client side**: if `soundTick >= soundTickTarget`: `soundTickTarget = nextInt(80) + 20` (20..99),
`soundTick = 0`, `Level::playSound(region, mSmeltSoundEvent, block centre (+0.5 each axis), data -1, empty
entity id, false, false)`; then `soundTick += 1`, `lit -= 1`. Both counters start at 0 (constructor
1457f2fa0 zeroes `+0x4e4`), are not saved, so a furnace sounds on its first lit tick, also after a load.
Events from the constructor calls: furnace **283** (`FurnaceUse`), blast furnace **279**, smoker **278**
(container types 2 / 27 / 28, block actor types 1 / 38 / 39). sapling should send `LevelSoundEvent` with that id
to players in range (range rule: mem, same as other `playSound`).

## 4. Slot rules for ItemStackRequest

Transfer handler **1446d43e0** maps the screen simulation's result (148b76470) to statuses: 50
FailedToValidateDstSlot, 55 CannotPlaceItem (the destination refused), 54 CannotSwapItem, 65 CannotRemoveItem,
59 CannotDropItem, 60 CannotDestroyItem; also 47, 48, 49, 51 (names mem, numbers BDSRE). The destination's
validation vtable decides (`isValidSlot` +8, `isItemAllowedInSlot` +0x10, `getAvailableSetCount` +0x18; vtables
found by their `getContainerOffset`/`getContainerSize` constants, inf):
- **Fuel** (vtable 14a954010): `isItemAllowedInFuelSlot` **1457f3470**: allowed if `getBurnDuration(item, 200) >
  0`, or the amount is 1 and the item is `bucket` or `lava_bucket`; set count `getAvailableFuelSetCount`
  1457f35d0 (a bucket: 1). Anything else: **55**.
- **Ingredient** (14a954060): accepts anything.
- **Output** (14a953c00, offset 2 size 1): `isItemAllowedInSlot` returns true; **I found no refusal**. Placing
  into furnace_output may well succeed on BDS (inf; see Not settled).
- Hoppers (`canPushInItem` 1457f3e10): ingredient only from face 1 (up), fuel from any other face and only fuel.
- Wet sponge: when the input is `wet_sponge` and the fuel slot holds an empty `bucket`, a smelt turns the bucket
  into the item at 14ccc95f8 (water bucket: inf) (tick 1457f4630).

## 5. Saving and resuming

- `load` **1457f3ed0** / `save` **1457f4220** (vtable 14a785cd0, slots 1-2): `BurnTime` short -> lit time
  (+0x288), `CookTime` short -> cook (+0x290), `BurnDuration` short -> lit duration (+0x28c), `StoredXPInt`
  int (+0x294; if absent, old `StoredXP` short), `Items` list of compounds with `Slot` byte (slots > 2 ignored).
  A wrong tag type reads as 0 (getters check type 1/2/3). Nothing else is saved (no sound counters, no last fuel).
- `onCustomTagLoadDone` **1457f41e0**: sets `mNoDrop`, calls `FurnaceBlock::setLit(BurnTime > 0, ...)`
  **147163930** so the block matches the saved fire, clears `mNoDrop`.
- `tick` has no viewer check: a loaded, lit furnace keeps burning and smelting with nobody looking. That block
  actors tick every tick in ticking chunks is mem.
- Tick order (1457f4630): `canBurn` (1457f56b0) once at the start; lit: sound, `lit -= 1`; if `lit == 0` and
  canBurn: refuel (copy to `mLastFuelItem`, use one, lava bucket leaves a bucket); if canBurn and lit:
  `cook += 1`, at `interval` smelt and `cook = 0`; else `cook = clamp(cook - 2, 0, interval)`; lit flip:
  `setLit` with `mNoDrop` set.

## 6. Lit blocks

`minecraft:lit_furnace`, `minecraft:lit_blast_furnace`, `minecraft:lit_smoker` are in the current exe
(VanillaBlockTypeIds init 1425f41e0, refs 142605d1e, 142605c64, 142605e92).

## For sapling

- Replace `furnaces/fuel.rs`: rule table 1a-1i keyed by block name / item name; Wood material needs a block list
  (planks, logs, wood, stripped, slabs, stairs, fences, gates, doors, trapdoors, plates, wooden workstations...).
  Drop wool and carpets; slabs 1.5; bowl 1; hanging signs 4; truncate, do not round.
- XP: compute on the take, from the taken count; ignore stored XP except to withdraw what a loaded world holds.
- Send LevelSoundEvent 283/279/278 from the furnace tick; tick every loaded furnace, not only watched ones.

## Not settled

- Placing into furnace_output: no refusal found in validation; confirm on a live BDS before copying.
- Material of logs, chests, bookshelf, lectern, composter, barrel, loom, jukebox, note block, tables, stairs,
  fence gates (registered outside 14849c7e0). Expected Wood (1.5).
- The `Button` bit 0x100 is from LeviLamina's `BlockProperty` (may drift); the 0.5 rule needs it.
- The 0.7 XP item after `iron_door` (redstone?), and the water-bucket name at 14ccc95f8.
- Orb position on break (block corner vs centre) and the player-position source in 148b7f020.
- Whether `getFurnaceXPmultiplier` is overridden by any item class (default assumed `Item+0x1b4`).
- **Conflict with hoppers.md** (item 6): there a hopper pulling the result adds its smelt XP x 100 to the
  furnace's stored XP (1429cfa55, 1457f60a0), which section 2's byte scan did not find (another offset base?).
  sapling follows hoppers.md: what hoppers take is owed to the next player.
