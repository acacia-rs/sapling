# BDS hoppers: ground truth for sapling (2026-10-06)

Tags as in `containers-crafting.md`: **REC**, **DATA**, **BDSRE** (current exe), **BDSRE-old**, **Java**, **mem**, **inf**.
Nothing here was tested on a live server. No hopper traffic has been recorded.

- Exe: `C:/Users/Hexay/acacia/acacia/.testserver/bds-molang/bedrock_server.exe` (bdsre cache `6ab54bee`; the
  default `.testserver/bds/` path does not exist). Run `BDS_EXE=<that> py -3 C:/Users/Hexay/acacia/bdsre/bdsre.py c <va>`.
- **No BDSRE-old in this pass**: `bdsre/builds/1.21.3.01/` is missing and no `bedrock_server.pdb` exists on the
  machine. The names below are LeviLamina `HopperBlockActor.h`/`Hopper.h`/`Container.h` names, matched by
  behaviour, field offsets and vtable slots. They are not pdb names.
- Layout used to read the code: actor `+8` pos, `+0xf0` Container, `+0x288` Hopper {`mCooldownTime`,
  `+0x28c` `mMoveCooldownTime`, `+0x290` `mTransferedFromChestMinecart`, `+0x291` `mIsEntity`, `+0x294`
  `mMoveItemSpeed`}, `+0x318` 5 x ItemStack (0x98 each), `+0x610` `mLastTick`. Container `+8` is ContainerType.

## What sapling needs, in order

1. **Tick**, `HopperBlockActor::_tick(region, maxRecursion)` 1457f82a0; the tick component's entry 1457f81e0
   calls it with **50** (`mov r8d, 0x32`). The entry returns early on the client, and also when the dimension
   reports `[+0x20c] < [+0x208]` (meaning unknown). Order inside `_tick` (BDSRE):
   1. `if level.currentTick <= mLastTick: return; mLastTick = currentTick`, so it runs at most once per tick.
   2. `old = cooldown; cooldown -= 1; if old > 1: return; cooldown = 0`. The countdown also runs while the hopper
      is powered.
   3. If `maxRecursion >= 1`, it first ticks (recursion - 1) up to three **downstream hoppers**: the hopper at
      y-1, the hopper below the facing target, and the hopper below the paired half if the target is a paired
      chest. Their order is shuffled with two level-RNG draws. `mLastTick` keeps any of them from running twice.
   4. **Powered: if `toggle_bit` != 0, return**. Nothing moves, and that includes item pickup (inf: the second
      hopper state hash 14cd24d68 next to `facing_direction` 14cd26368).
   5. `Hopper::_tryMoveItems(region, container, pos, facing_direction, canPush=true)` 1429ce8c0. On success it
      sets the container-changed flag (`+0x4c |= 1`).
2. **`_tryMoveItems`** 1429ce8c0 (BDSRE):
   - **Push first.** The push runs only if some slot is non-empty: `_pushOutItems(region, c, pos, facing)` 1429ceb80.
   - Clears `mTransferedFromChestMinecart`.
   - **Then pull**, only if some slot is empty or below its item's max stack:
     `_tryPullInItemsFromAboveContainer` 1429cf550, and **only if that fails** `_tryPullInItemsFromWorld` 1429cfd10.
   - If the push or the pull moved something: `mCooldownTime = mMoveItemSpeed` and it returns true. Otherwise the
     cooldown stays 0, so it tries again every tick.
   - **`mMoveItemSpeed` = 4**. The ctor 1457f7d20 writes `+0x294 = 4` and `mCooldownTime = mMoveCooldownTime = -1`.
     That gives one move per 4 game ticks if block actors tick at 20 Hz (see Not settled). `TransferCooldown`
     loads into `mCooldownTime`.
   - A tick can therefore move up to **one item out and one item in**.
3. **Received-item cooldown.** In `_tryMoveInItem` 1429cd870, when the target is a hopper (ContainerType 8) and it
   holds exactly 1 item afterwards (`_countItems` 1457f92e0 sums the counts, so it was empty before), the target
   gets `cooldown = mMoveItemSpeed + (target.mLastTick < now ? 1 : 0)`, which is 5 if the target has not ticked yet
   this tick and 4 if it has. `_pushOutItems` repeats this with a literal 4. So it does not have Java's 8/7 quirk:
   either way the target waits 4 ticks after the current one.
4. **Push**, `_pushOutItems` 1429ceb80 (BDSRE):
   - Target = pos + offset[facing], with x/y/z byte tables at 14a7c2644/263e/2638. The face passed to the target
     is `opposite[facing]` (byte table 14a682118 = `1,0,3,2,5,4`).
   - **facing_direction: 0 down (0,-1,0), 1 up (0,1,0), 2 north (0,0,-1), 3 south (0,0,1), 4 west (-1,0,0),
     5 east (1,0,0)** (DATA from the exe: the Vec3i table at 14a712760 matches). A value of 6 or more pushes
     nowhere.
   - Containers at the target come from `_getContainersAt` 1429d0260: first the block's container
     (`_getContainerInBlock` 1429cfe50), then every entity in the 1x1x1 block AABB whose `ContainerComponent` has
     its flag byte `+0x1d0` set (chest and hopper minecarts and so on: just noted).
   - For each container: `initializeContainerContents(region)` (vt+0x140, which fills loot tables). Skip the
     container if every slot is full. Otherwise, for **hopper slot 0..4**, the first non-empty one: copy it and
     try **target slots 0..n in order**, `_tryMoveInItem(..., count 1, face)`. The first success decrements the
     hopper slot and calls `setContainerChanged` on both sides, then returns true.
   - facing 0 with a **composter** below (block id 14ccdf110, inf: same helpers as the composter code
     148b34c90/148b34a90): each slot in turn feeds 1 item to the composter instead.
5. **`_tryMoveInItem`** 1429cd870 (BDSRE): `target.canPushInItem(slot, face, item)` (Container vt+0x100) must
   pass. A non-empty slot takes the item only if it is the same item and below max stack, filled up to
   `min(count, max - have)`. An empty slot takes `count`. Target slots are tried in index order and the first
   one that accepts wins, whether it is empty or a stack.
6. **Pull from above**, 1429cf550 (BDSRE):
   - Composter above (inf): take its output (bone meal) into the first accepting hopper slot. Done if that works.
   - Otherwise take the containers at pos+(0,1,0) as in step 4. For each one, source slots 0..n: the first
     non-empty slot whose `canPullOutItem(slot, face 0, item)` (vt+0x108) passes and whose item fits a hopper
     slot (slots 0..4 in order, count 1) is moved. That is one item per pull.
   - Hopper minecart special case: a Dispenser-type source with byte `-0x104` == 0x21 skips `canPullOutItem`.
   - Source ContainerType 2/27/28 (furnace, blast furnace, smoker): the item's smelt XP x 100 goes into the
     furnace's stored XP (`+0x1a4`; 1429cfa55, `1457f60a0`). A chest minecart or chest boat source sets
     `mTransferedFromChestMinecart`.
7. **Item entities**, `_tryPullInItemsFromWorld` 1429cfd10 -> `_tryAddItemsFromPos` 1429cdfe0 (BDSRE):
   - Skipped entirely when the block above returns true from BlockType vt+0xb8. HopperBlock's slot returns true
     there; inf: `isContainerBlock`, since the header index is off by one. A plain solid block above does **not**
     stop it.
   - **AABB** (block hopper): x `[x, x+1]`, y `[y+0.625, y+2]`, z `[z, z+1]`. A minecart hopper uses x/z +-0.5
     around its own position.
   - The query is for actor type 0x40 (item). Entities already removed are skipped.
   - The first item entity that fits is inserted with **its whole count**: first accepting hopper slot, then
     one more pass for the remainder. The entity is removed at count 0. At most one entity per pull.
   - Runs only on a move tick: cooldown done, not powered, not full, and the pull from above failed.

## Container rules (canPushInItem / canPullOutItem; face = the target's face touching the hopper)

Block actors that `_getContainerInBlock` 1429cfe50 accepts (BlockActorType): Furnace 1, Chest 2 (trapped chest
too, mem), BrewingStand 8, Dispenser 13, Dropper 14, Hopper 15, ShulkerBox 25, Jukebox 33, BlastFurnace 38,
Smoker 39, Barrel 42, ChiseledBookshelf 51, DecoratedPot 53, Crafter 55, Shelf 59. **Ender chest is not
accepted** (its Container vtable 14a878d10 also returns false for both rules). Double chest: `getContainerSize`
141f30340 returns **54 when paired, else 27**, so a hopper sees 54 slots. Slot order is in
containers-crafting.md B.

The Container base and the Hopper return true for both rules (1400a2880). Overrides, with Container vtables found
through the shared slot-31 entry (BDSRE):

- **Furnace / blast furnace / smoker** (one vtable, 14a785ec0; inf: all three use it):
  - push 1457f3e10: **slot 0 only from face 1** (a hopper above), any item, with no smeltability check;
    **slot 1 from any face except 1, only if `burnDuration(item) > 0`**; slot 2 never.
  - pull 1457f3e60: **slot 2 from face 0**; **slot 1 from face 0 only if the item is the bucket** (inf: the same
    item global the furnace tick looks up at 1457f4dcb); slot 0 never.
- **Brewing stand** (14a7862f0):
  - push 141f219d0: slot 0 ingredient from face 1 if it is a brewing ingredient (14204ae40); slot 4 fuel from
    face != 1 if it is fuel (14204a9d0, blaze powder); slots 1-3 from face != 1 if it is a potion or glass
    bottle and the slot is empty.
  - pull 141f21ce0: slots 1-3 from face 0.
- **Shulker box** (14a841f50): push 1448820e0 refuses items whose block is in a list at 14cce30b8 (inf: the
  shulker boxes). Pull always.
- Crafter push 1457e8970: balances, refusing a slot when another enabled slot of the same item holds fewer.
  Jukebox (inf) push 1457fd970: music discs only; pull 1457fd9b0 returns a flag. Chiseled bookshelf (inf) push
  141f367b0: empty slot and a tagged item; pull: non-empty slot.

## Save data and client sync

- `HopperBlockActor::load` 1457f8a80 / `save` 1457f8cf0 (BDSRE). Base `BlockActor::save` 14486c2f0 writes
  `CustomName`, plus `FilteredCustomName` when a filtered name exists (and `id`, x/y/z, isMovable: mem).
- **`Items`**: a list of item compounds, each with **`Slot` as a Byte** (14284e950 stores tag variant 1), empty
  slots skipped. Load ignores a Slot above 4.
- **`TransferCooldown`: Int** (14284eb70, the same putter as x/y/z).
- `_getUpdatePacket` 1457f0280 sends `BlockActorData` **only when the hopper has a custom name**, and its payload is
  only the name fields (14488aca0). **Items are never sent to clients**; they see them only through an open
  container window.

## Events and sounds

- No sound or level event in the move path (BDSRE: none of 1429ce8c0/ceb80/cd870/cf550/cdfe0 builds one).
- Two telemetry or achievement events through 142d3c8f0: id 0x34 when a push into a chest-type container follows
  a pull from a chest minecart, and id 0x35 from `onNeighborChanged` 1457f9480 when 3 or more hoppers feed the
  same furnace. sapling can skip both.
- Composter interactions may emit the composter's own events (not checked).

## Ticking scope

- The hopper ticks as a block actor through its tick component (vtable `+0x40` = 14a879440). inf/mem: only in
  chunks the level ticks (within simulation distance or a ticking area), like every other block actor. Not
  traced.
- Hopper minecarts run through `HopperSystem::tick` 148489110 -> 148489a30, which uses the same `Hopper`
  code with callbacks (`_tryMoveItems` callbacks overload 1429ce660). Just noted.

## Not settled

- **Real move rate.** The code gives 4 ticks between moves, which is 5 items/s at 20 Hz block-actor ticking.
  Folk knowledge says 2.5 items/s (8 ticks). Confirm with a REC (a timed hopper -> chest) before trusting 4.
- The meaning of the dimension check `[+0x20c] < [+0x208]` in the tick entry 1457f81e0.
- Composter, bucket and shulker-list identities are inferred from context, not from a string.
- Item pickup delay: no check is visible in 1429cdfe0 besides the removed flag. A freshly dropped item may be
  pulled at once.
- Whether the blast furnace and smoker really share furnace vtable 14a785ec0 (one container vtable found).
- BDSRE-old cross-check was impossible (no 1.21.3.01 exe or pdb on disk).
