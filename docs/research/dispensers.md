# Bedrock dispensers, droppers, crafters, from BDS (bdsre)

Read 2026-10-25 from `acacia/.testserver/bds-blocks/bedrock_server.exe` (PE stamp 6ab54bee, the build of
`redstone.md`) with bdsre (`c|dis|win|vt|refs|callers|imm`), vtable slots from small pefile scans of `.rdata`,
layouts and enums from LeviLamina (`LevelSoundEvent.h`, `LevelEvent.h`, `BlockActorType.h`, `GameRules.h`,
`Container.h`, `BucketFillType.h`). Projectile power/uncertainty come from the vanilla behaviour pack's entity
JSON (`behavior_packs/vanilla*/__brarchive/entities.brarchive`), not the binary. No `name` labels added.
**Everything is [decompile]** unless marked; nothing here is drilled yet.

Notation: Facing 0 down, 1 up, 2 north(-z), 3 south(+z), 4 west(-x), 5 east(+x). `facing_direction` maps to
Facing 1:1 (byte table 14a8e43e0). Step tables: x 14a7c2644 `[0,0,0,0,-1,1]`, z 14a7c2638 `[0,0,-1,1,0,0]`
(y at 14a7c263e); float direction vectors 14a682180 (12 bytes per Facing). Sounds are `LevelSoundEvent`
(363 `BlockClick`, 364 `BlockClickFail`, 479 `CrafterCraft`, 480 `CrafterFail`), sent via
`Level::broadcastSoundEvent(region, id, pos, block/extra, ...)`; level events are `LevelEvent`.

## 1. Blocks and redstone

| VA | Suggested label | What |
|---|---|---|
| 148a20800 | `DispenserBlock::DispenserBlock` | vt 14a93d390; subscribes interact 148a20c20, queued tick 148a21120, redstone 148a214f0 |
| 148a2cc00 | `DropperBlock::DropperBlock` | vt 14a93f5c0; same subscriptions; differs from dispenser only in vt[133] |
| 140a20220 | `DispenserBlock::getTickDelay` (vt[132]) | `return 4` (shared) |
| 148a22930 | `DispenserBlock::dispenseFrom` (vt[133]) | §2 |
| 148a2cce0 | `DropperBlock::dispenseFrom` | §3 |
| 148a214f0 | dispenser `_onRedstoneUpdate` | D1 |
| 148a21120 | dispenser queued tick | `if !clientSide: this->dispenseFrom(region, pos)` |
| 148a23170 | `DispenserBlock::getDispensePosition` | D4 |
| 148a233a0 | default dispense (drop 1) | D6 |
| 148a235c0 | `ejectItem(region, pos, face, item, count)` | D5 |
| 148a22840 | `DispenserBlock::getComparatorSignal` (vt[91]) | D31 |
| 14713f050 | `CrafterBlock::CrafterBlock` | vt 14a8e0270; interact 14713f470, tick 14713f800, redstone 147140df0 |

- D1 Redstone (148a214f0; crafter 147140df0 identical): `signal > 0 && !triggered_bit` → (server, or region
  flag) queue a tick at pos, delay `getTickDelay()` = **4** (crafter passes literal 4), priority 0; then
  `setBlock(triggered_bit=1, flags 3)`. `signal <= 0 && triggered_bit` → `setBlock(triggered_bit=0, flags 3)`.
  Nothing else on the falling edge; the queued tick still fires. No first-time skip in the handler.
- D2 Queued tick: dispenses unconditionally (does not re-read power or `triggered_bit`). A short pulse fires once.

## 2. Dispenser: dispenseFrom (148a22930)

- D3 Block actor must be type 13 (Dispenser; `isTypeOrDerived` 1457f0030 = `t == 13 || own == t`). First the loot
  table is unpacked into the container (144878780). Slot pick: walk slots 0..size-1; for each non-empty slot,
  `k += 1` (k starts 1) and `if level.getRandom().nextInt(k) == 0: chosen = slot` (Random vt+8; reservoir
  sampling: **uniform over non-empty slots**, one `nextInt` per non-empty slot). Empty → `BlockClickFail` 364 at
  the dispenser block pos (extra = dispenser block), plus game event 14cc89850 at pos; nothing else.
- D4 Dispense position: `(x + 0.5 + 0.7·sx, y + 0.3 + 0.7·sy, z + 0.5 + 0.7·sz)` from the block pos and the facing
  step (so horizontal y + 0.3, down y − 0.4, up y + 1.0). Behaviours get this Vec3 and the Facing.
- D5 `ejectItem` (148a235c0): copy of the item with `count = min(count, stack)`; spawn `ItemActor` (spawner
  vt+0xa8) at `(px, py + (face >= 2 ? −0.15625 : −0.125), pz)`; `speed = levelRandom.nextFloat()·0.1 + 0.2`;
  `vel = (speed·sx, 0.2, speed·sz)` (y is **0.2 for every face**, Java's shape); then each axis
  `+= nextGaussian()·0.0075·6` (0.045; MT `genrand_int32` polar Box-Muller with a cached second value).
- D6 Default behaviour (148a233a0): `ejectItem(count 1)`; `container.removeItem(slot, 1)`; `BlockClick` 363 at the
  dispense pos, extra = block at floor(dispense pos).
- D7 Choice of behaviour by item (`item->mLegacyBlock` at Item+0x178):
  - not a block item → `Item::dispense(region, container, slot, pos, face)` (Item vt[85]); false → D6.
  - block item with tag 14ccdb7d0 → Item::dispense (same); tag not identified.
  - block item that is one of 12 `minecraft:sulfur_cube_archetype_*` items → equip onto a mob in front (D16,
    swap allowed); out of scope.
  - other block item: `BlockType vt+0x188` (canDispense; base true, TNT overrides) then `BlockType::dispense`
    (vt+0x190); false → D6.
- D8 After any behaviour returns true: `LevelEvent 2000 ParticlesShoot` at the dispense pos, data
  `(sx+1) + 3·(sz+1)` (4 for up/down); block actor marked changed; `container.setContainerChanged(slot)`.
  Behaviours that "fail" but return true (bone meal, flint and steel, shulker box) still get the smoke.

## 3. Dropper: dispenseFrom (148a2cce0) and push (1457f0dd0)

- D9 Block actor type 14. Slot pick as D3 (no loot unpack here). Empty → `BlockClickFail` 364 at block pos
  only (no game event).
- D10 Non-empty: `_tryMoveInItem` (1457f0dd0). If it returns false and the item is valid: D6 at the dispense pos
  (`BlockClick` there) and **a second `BlockClick` 363 at the dropper's block pos**; no `ParticlesShoot`. Then
  changed + `setContainerChanged`.
- D11 `_tryMoveInItem`: target = `getContainerAt(region, pos + facing)` (1457ea870): block actor types Furnace 1,
  Chest 2, BrewingStand 8, Dispenser 13, Dropper 14, Hopper 15, ShulkerBox 25, Jukebox 33, BlastFurnace 38,
  Smoker 39, Barrel 42, ChiseledBookshelf 51, DecoratedPot 53, Crafter 55, Shelf 59; else the first entity in the
  1×1×1 cell with an active `ContainerComponent` (chest/hopper minecart, etc.).
  - container found: `initializeContainerContents` (loot); **a fresh random slot pick** (D3 rule, second RNG
    pass); copy that stack; for target slot 0..n-1 call 1457f1600(target, item, slot, face = opposite(facing))
    until one accepts; write the remainder back to the dropper slot. Returns **true even if nothing fits** →
    full target: nothing happens, no drop, no sound.
  - 1457f1600 (add one item to a slot): `target.canPushInItem(slot, face, item)` must be true; empty slot → put a
    1-count copy; same stackable item below max stack → +1; else reject. Source count −1. If target container
    type is Hopper (`container+8 == 8`) and its cooldown < 1 → cooldown = 4. `setContainerChanged(slot)`.
  - no container, block in front is `minecraft:composter`: facing **down only** → random slot pick, then
    `ComposterBlock::addItems(item, 1, ...)` (148b34c90), remove what was used; returns true. Composter on any
    other side: returns true and does nothing.
  - otherwise false → D10 drop.

## 4. Dispense behaviours (Item vt[85] overrides)

Projectile shoot: `Spawner::spawnProjectile(region, type, owner=null, pos, dir)` (14118d160) creates the actor
at the dispense pos and calls `ProjectileComponent::shoot(dir, power=comp+0x100, uncertainty=comp+0x1a0)`
(1424020c0): normalize dir (if len > 0.001), add `gaussian·0.0075·uncertainty` per axis, scale by power. Power and
uncertainty are the entity JSON's `minecraft:projectile` (`power`, `uncertainty_base`).

| Item (vtable) | dispense VA | Behaviour |
|---|---|---|
| base `Item` | 141a888b0 | wearable (`vt+0x138`) → D16 slot from item, no swap; honeycomb-like special (141a88af0, not read); else false |
| ComponentItem (data-driven) | 1420b7ed0 | `minecraft:projectile` → spawn component's entity, dir `(sx, sy+0.1, sz)`; remove 1; `BlockClick` + `LevelEvent 1002` data 78642; `minecraft:entity_placer` → 14302a440 (not read); `minecraft:wearable` → D16 with the component's slot |
| arrow / tipped (14a7c9c20) | 142a7d820 | actor 0xc00050 Arrow (142a7cf70, copies item data via 142a7d140), dir `(sx, sy+0.1, sz)`; remove 1; `BlockClick` 363; `LevelEvent 1002 SoundLaunch` data 78642. JSON: power 1.6, uncertainty_base 16 (multiplier 4) |
| snowball (14a7e3530) | 142efcce0 | server only; actor 0x400051, dir `(sx, sy+0.1, sz)`; remove 1; LE 1002 data 78642 (no click). JSON power 1.5, gravity 0.03 |
| egg (14a806450) | 1436f5920 | actor 0x400052; same as snowball. JSON power 1.5 |
| splash potion (14a7e3d80) | 142efe4e0 | actor 0x400056 with the item; edu rule 40 off + potion flagged (vt+0x188) → false; else as snowball. JSON power 0.5, angle_offset −20 |
| lingering potion (14a76a120) | 141a613e0 | actor 0x400065 (141a60f10); dir `(sx, sy+0.1, sz)`; remove 1; LE 1002 data 78642. JSON power 0.5 |
| experience bottle (14a807a40) | 141a613e0 | actor 0x400044 (1436f80c0); as lingering. JSON power 0.5 |
| fire charge (14a807e60) | 1436f8c40 | actor 0x40005e SmallFireball (1436f8b40); dir `= step + gaussian·0.05` per axis (thread-local Random); remove 1; `LevelEvent 1008 SoundGhastFireball` data 78642. JSON power 1.3, uncertainty_base 10 |
| ice bomb (edu) | 141a828f0 | as fire charge; sound `Throw` 40 |
| trident | 141fa7b30 | spawn 0xc00049, damage 1, out of scope |
| wind charge | ComponentItem path (inferred) | JSON `wind_charge_projectile` (vanilla_1.21.0): power 1.0, uncertainty_base 1.0 |
| bucket (14a9092c0) | 147a9b070 | D12 |
| bone meal / rapid fertilizer | 1436f8210 | D13 |
| flint and steel | 141a7a660 | D14 |
| shears | 142ef54b0 | fetch entities in the cell, `minecraft:on_sheared` / loot `*_shear.json` (sheep, mooshroom, snow golem, bogged), sound `Shear` 45; beehive path via 142ef52d0 with `BlockClick`; not read in detail; built for sheep and mooshrooms (`creatures::shear_in`) |
| potion (water bottle) | 141a717e0 | D15 |
| glass bottle | 147a937f0 | D17 |
| humanoid armor / elytra | 141a81110 | D16 slot `item.mArmorSlot + 2`, no swap |
| shield | 142ef6e00 | D16 slot 1 (offhand) |
| skull | 142efb5e0 | wither skeleton skull into air completing a wither → place skull, spawn wither, `BlockClick`; else D16 head (2) |
| horse armor family (14a76ba40, 14a770430) | 141a6b300 | into the slot of an entity `ContainerComponent` in the cell; `EquipGeneric` 96 + `BlockClick` |
| saddle, harness, wolf armor, carpet, chest | 142ef1960, 141a7f810, 1420b7ed0, 141a3ef60, 148b26600 | equip/`on_saddled`/`on_harnessed`/`on_chest`; out of scope |
| spawn egg (14a7c93b0) | 142a788e0 | D18 |
| boat, minecart, firework, brush | 141a6af10, 142a863f0, 1436fe320, 147a951b0 | out of scope; the firework is built since (`fireworks::dispense`, not read here) |

- D12 Bucket (147a9b070), `mBucketType` at Item+0x218 (0 empty, 1 milk, 2–5 fish, 8 water, 10 lava, 11 powder
  snow, 12 axolotl, 13 tadpole):
  - milk → false (default drop).
  - empty: block (liquid layer) at the cell; must be a source (`liquid_depth == 0`, 14cd25490) or powder snow;
    material water → `water_bucket`, lava → `lava_bucket`, powder snow → `powder_snow_bucket`, else false.
    Remove the liquid (liquid layer cleared if it is the extra block, else `setBlock(air, 3)`); a
    `bubble_column` there → air too. `BlockClick` 363 at pos, game event 14cc8a91c (fluid pickup),
    `removeItem(slot, 1)`, then `addItemToFirstEmptySlot(filled)`; no empty slot → `ejectItem(count 1)` (D5).
  - full: lava (10) needs edu rule 40 (`AllowDestructiveObjects`) true. Places water for 2–5, 8, 12, 13; lava for
    10; powder snow for 11 via 147a9a120 (also spawns the fish, not read). Success → `BlockClick` (extra 1000),
    remove 1, empty bucket via `addItemToFirstEmptySlot` or eject. Failure → false → default drop of the full
    bucket.
- D13 Bone meal (1436f8210): rapid fertilizer needs education. `block.onFertilized(region, cell, null, type)`.
  Fail → `BlockClickFail` 364 (extra 1000) and **true** (no drop, smoke still). Success → `BlockClick`,
  `LevelEvent 2005 ParticleCropGrowth` at the cell, count −1.
- D14 Flint and steel (141a7a660), block in the cell: air (14ccf9950, inferred) → fire (14ccdb710, inferred),
  flags 3, game event block_place, no survival check; TNT (14cce2cf8, inferred) → set `explode_bit`, call its
  destroy (primes) and set air; `campfire`/`soul_campfire` → light (141a3a160); candle → 141a3c820; candle
  cake → 141a3e1a0. Success: durability −1 (141a56c80), `setItem`, `BlockClick`, game event. Anything else:
  `BlockClickFail` 364, **true**.
- D15 Water bottle (141a717e0): only if the potion is water and the cell is a dirt kind (list 14ccdb858) or
  `dirt_with_roots` → `mud`, flags 3, game event, `BlockClick`, remove 1, glass bottle via
  `addItemToFirstEmptySlot` or eject, then sound 26 and a particle event 25. Otherwise false → default drop.
- D16 Equip (1436f3c10): entities (`fetchEntities` type 0x100) in floor(dispense pos)'s 1×1×1 box passing a
  filter; slot < 2 hand container, 2..6 armor (slot − 2). Target slot empty → equip one, persistent flag,
  remove 1, `BlockClick` at dispense pos. Occupied: no swap → next entity / false; swap (sulfur cube path)
  drops the old item first. All fail → false → default drop.
- D17 Glass bottle (147a937f0): `beehive`/`bee_nest` with full honey → `honey_bottle`, reset honey (142cddf90);
  water source (material water, depth 0) → `potion` (water). Each: `BlockClick`, remove 1,
  `addItemToFirstEmptySlot` or eject; game event fluid pickup. Else false → default drop.
- D18 Spawn egg (142a788e0): actor spawned near the cell: per axis `floor + 0.5 + s·0.5 + s·halfExtent` (x/z by
  half width, y by half height); persistent; item custom name applied; `BlockClick`; game event. Some type
  gets a 5% swap (zombie → zombie_villager?). Exact y for up/down not read.

Block behaviours (BlockType vt+0x190):

- D19 TNT (1471f9bd0; canDispense 1471fa120 = gamerule 21 `tntexplodes`): spawn PrimedTnt (type 0x41) at
  `floor(dispense pos) + 0.5` on each axis (centre of the cell); `underwater_tnt` sets the underwater flag
  (1471f9090); `BlockClick`, `LevelEvent 1005 SoundFuse`, game event, remove 1. Fuse and launch velocity are the
  entity's defaults, not set here. Gamerule off → default drop of the TNT item.
- D20 Shulker box (148b544d0): cell must be air or water and inside the height range; facing = dispenser facing
  unless the cell beyond is not solid, then the first of faces 0..5 with a solid neighbour and a free opposite
  (skip down onto 14ccdd3a0); place the box (item aux → colour) flags 3, copy the item's contents, block actor
  facing = `opposite(face)`, remove 1. No sound. Cell blocked → **true**, nothing placed, item kept.
- D21 Glowstone (1471648b0): cell is `respawn_anchor` → charge (144bcb1c0), remove 1, `BlockClick`; else false.

## 5. Crafter

| VA | Suggested label | What |
|---|---|---|
| 14713f800 | crafter queued tick (craft) | D23–D26 |
| 1457e92e0 | `CrafterBlockActor::getCraftResults` | recipe lookup |
| 1457ea000 | `CrafterBlockActor::_tryMoveInItems` | output into container in front |
| 1457e9a20 | `CrafterBlockActor::setCrafting` | `crafting=1` (flags 2), ticks 6 |
| 1457e9cf0 | crafter tick component | count down, clear `crafting` |
| 1457e8970 | crafter `canPushInItem` (Container vt[32]) | D28 |
| 1457e8b40 | crafter `isSlotDisabled` (vt[42]) | `slot >= 9 || mask>>slot & 1` |
| 147141810 | `CrafterBlock::getComparatorSignal` (vt[91]) | D31 |
| 147141240 | `CrafterBlock::getVariant` | 2 if crafting, else triggered |
| 1457e8ca0 / 1457e8f20 | load / save | D29 |
| 1457e8410 / 1457e8830 | `_getUpdatePacket` / `_onUpdatePacket` | D29 |

- D22 Redstone: D1 exactly (rising edge with `triggered_bit`, tick delay 4).
- D23 Tick: server only; block actor type 55. Copy the 9 slots into a 3×3 `CraftingContainer`; results =
  1457e92e0: empty grid → none; else the first matching recipe among those with tag 14cde8268 (inferred
  `crafting_table`), results = its full result list (Bedrock recipes list remainders as results: cake gives
  `cake` + `bucket ×3`).
- D24 No results → `CrafterFail` 480 at the crafter pos (extra = crafter block). Nothing consumed, no state change.
- D25 Results → 1457ea000: front = Facing from `orientation` (14a8e740c: 0–3 down, 4–7 up, 8 west, 9 east,
  10 north, 11 south). Container in front (D11 list) → loot init, then for each result insert one at a time with
  1457f1600 (face = opposite), slots scanned 0..n-1 per item, until the stack is gone or nothing accepts; empty
  results are dropped from the list. No container and facing down onto a composter → `addItems(stack)`.
  Then:
  - all results placed → **no sound, no particle**.
  - leftovers → for each: `ejectItem(stack, full count)` (D5) at `(x+0.5+0.7sx, y+0.3+0.7sy, z+0.5+0.7sz)`,
    `LevelEvent 3609 ParticlesShootWhiteSmoke` at that pos with data `(sx+1)+3(sz+1)`, and `CrafterCraft` 479 at
    the crafter pos (per item for the event; sound once after the loop). Players within 8.5 get a trigger.
- D26 Consume: for every non-empty slot `removeItem(slot, 1)`; block actor changed; `setCrafting`: if
  `crafting` is 0 set it to 1 (flags 2); `crafting_ticks_remaining = 6`.
- D27 Tick component (1457e9cf0): `if ticks >= 1: ticks -= 1; if it was 1 and crafting: crafting = 0 (flags 2)`.
  So `crafting` shows for 6 ticks after a craft.
- D28 `canPushInItem(slot, face, item)` (any face): disabled → false; slot empty → true; same item below max
  stack → false if any later enabled slot is empty or holds the same item with fewer; else true (different item
  or full also returns true; the add itself then fails). With hoppers scanning slots 0..8 this puts the item in
  the enabled slot with the fewest of it, earliest on ties, empty counting as fewest. `canPullOutItem` → true.
- D29 NBT: save = base filling-container save (`Items`: list of item compounds + byte `Slot`, non-empty only;
  `LootTable`, `LootTableSeed` when set; `CustomName`) + `disabled_slots` (short bitmask, 9 bits; load masks
  0x1ff). `crafting_ticks_remaining` (int) is **only in the block-actor update packet** (with `disabled_slots`),
  not saved. `triggered_bit`, `crafting`, `orientation` are block states.
- D30 Toggle slot: `PlayerToggleCrafterSlotRequestPacket` (id 306, LeviLamina: `BlockPos mPos`, `int mSlotIndex`,
  `bool mIsDisabled`); server handler not read. Sound `CrafterDisableSlot` 483 exists.

## 6. Hoppers, NBT, comparators

- D31 Comparators: dispenser/dropper (148a22840, dropper passes the type-13 check): loot unpack, then
  `floor(14 · Σ(count/maxStack) / size) + (any non-empty ? 1 : 0)` — the standard formula. Crafter (147141810):
  `popcount(disabled_slots)` = 9 → 9; else **non-empty slots + disabled slots** (0..9).
- D32 Dispenser/dropper block actors (vt 14a878520 / 14a878840; load 1457e8d20, save 1457e8f80): `Items` list with
  `Slot` byte (slots > 8 ignored on load), `LootTable`/`LootTableSeed`, `CustomName`. Their container vtable
  (14a8786e0, inferred to be theirs): `canPushInItem` and `canPullOutItem` both true for every slot and face,
  `isSlotDisabled` false. Which hopper faces push/pull is the hopper's rule, not theirs.
- D33 Containers given things by a dropper or crafter that are hoppers get a 4-tick cooldown if theirs was < 1.

## Unverified

- Real-play checks: none of this is drilled. First targets: the 4-tick delay; dispense pos y + 0.3; the dropper's
  double click and missing smoke; dropper into a full chest doing nothing; crafter output into a container
  making no sound; `crafting` lasting 6 ticks; crafter comparator.
- `Random` vt+8 is `nextInt(bound)` and vt+0x28 `nextFloat` (inferred from use).
- Names of hash globals 14ccf9950 (air), 14ccdb710 (fire), 14cce2cf8 (tnt), 14ccdafe8 (lava), 14ccdd3a0, the dirt
  list 14ccdb858, tag 14ccdb7d0, recipe tag 14cde8268, game events 14cc89850 / 14cc8a91c / 14cc8a9ec.
- Arrow pickup flag (142a7d140 not read); arrow uncertainty: whether `uncertainty_multiplier` × difficulty is
  subtracted before comp+0x1a0; snowball/egg default uncertainty. sapling uses 8 for arrows (16 less 4 × normal
  difficulty) and Java's 6 for snowballs, eggs, potions and bottles (`shots/dispensed.rs`).
- Wind charge's dispense path (assumed ComponentItem projectile).
- Spawn egg exact position for up/down and the 5% branch; shears details; `Item::dispense` special at 141a88af0.
- Crafter toggle-slot server handling (empty slot required? sound?), interact 14713f470.
- `isFirstTime` handling for a dispenser placed already powered (consumer flags not read).
- Dispenser/dropper container vtable attribution (D32) and whether a dropper's `Items` differ.
