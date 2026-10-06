# BDS experience orbs and player XP: ground truth for sapling (2026-10-06, BDS 1.26.5x, protocol 2193)

Tags as in `containers-crafting.md` (REC, DATA, BDSRE, BDSRE-old, mem, inf). Nothing tested on a live server.

- BDSRE exe: `acacia/.testserver/bds-molang/bedrock_server.exe` (PE stamp 6ab54bee, the one bdsre's cache is
  for; the default `bds/` path is gone): `BDS_EXE=<that> py -3 C:/Users/Hexay/acacia/bdsre/bdsre.py c <va>`.
- The 1.21.3.01 pdb build is **not on disk** (`bdsre/builds/` is missing), so there is no BDSRE-old here. Names
  below are hand labels saved this pass (`names.user.json`), from LeviLamina headers + code shape.
- Protocol names: acacia-proto `AddEntity`, `RemoveEntity`, `MoveEntityDelta`, `SetEntityMotion`,
  `LevelEvent`, `LevelSoundEvent`, `UpdateAttributes`, `SpawnExperienceOrb`.

## 1. Spawning: `ExperienceOrb::spawnOrbs(region, Vec3 pos, int xp, DropType, Player* owner)` 142f9eb90

1. **Game-rule gate** by DropType (BDSRE): 0 NoType none; 1 FromBlock needs `doTileDrops` (rule 8);
   2 FromMob needs `doMobLoot` (6); 3 FromPlayer needs `keepInventory` (13) **false**. Rule ids: LeviLamina
   `GameRules.h`.
2. Server side (`!isClientSide`) only; the client-side branch sends `SpawnExperienceOrb` instead.
3. **Split, xp <= 12000** (BDSRE): loop `while xp > 0`, take the largest of
   **2477, 1237, 617, 307, 149, 73, 37, 17, 7, 3, 1** that is <= xp. Same thresholds as Java.
4. **Split, xp > 12000** (`0x2ee1 > xp` fails): 12 orbs of `xp/13`, then one of `xp - 12*(xp/13)`: exactly
   13 orbs. There is no other cap on the orb count.
5. **Merge on spawn**: each orb first tries `_tryMergeIntoExistingOrbs(region, pos, value)` 142f9f050:
   fetch `xp_orb` actors (type 0x45 = 69) in the box `[pos, pos+1]` (min corner = pos, not centred). Only if
   **13 or more** are there: spread `value` over all of them (`value/n` each, the remainder to the first),
   saturating at i32::MAX; no new orb. Else `_spawnOrb` 142f9f2d0: create actor type 69 at `pos`, set its
   value, add to the level.
6. **Initial motion** (orb `reloadHardcoded` 142f9d2e0, vtable slot 2 of 14a7e8a00, InitializationMethod 2):
   yaw = r*360; motion = ((r*0.2-0.1)*2, r*0.2*2, (r*0.2-0.1)*2), r = fresh random float each. i.e.
   x,z in [-0.2,0.2], y in [0,0.4]. Same as Java.
7. **Who calls it** (BDSRE, `callers 142f9eb90`):
   - **Furnace output taken**: `FurnaceOutputTakeXP::create` 142f482a0 builds the action only for a take from
     ContainerSlotType **26 `furnace_output`, slot 2**, with `xp = randomRound(count * xpPerItem)`
     (`FurnaceBlockActor::xpForTaken_randomRound` 1457f5ee0, per-item value from 1457f60a0). Its `award`
     148b7f020 (vtable 14a9536d0 slot 3) checks the player's open block actor is a furnace
     (`isType(1)`), adds `randomRound(StoredXPInt / 100)`, zeroes StoredXPInt, then
     `spawnOrbs(region, player StateVector pos, total, NoType, player)`. So orbs appear **at the player**
     (Actor+0x218 pos; for a player that is eye height, feet+1.62: mem), not at the furnace, no game rule.
   - **Furnace broken**: 1457f6f60 (`FurnaceBlockActor::spawnXPOnRemove`) spawns
     `randomRound(outputCount*xpPerItem) + randomRound(StoredXPInt/100)` at the **block's min corner**
     `(x, y, z)` as floats (no +0.5), DropType FromBlock, then drops the contents.
   - Others: ore/loot block break 141a23a70 (FromBlock), mob death 141b7fce0 (FromMob), player death
     1402bc2f0 (FromPlayer), villager trade 141baa8a0 (3+rand(4) per trade), grindstone 1446e3fe0,
     fishing 1429c6b50, a random-range spawner 142408580, and 140978420, which looks like the server handling
     a **client** `SpawnExperienceOrb` (pos at +0x30, count at +0x3c; inf). sapling should ignore that packet.
8. `randomRound(f)` (all furnace sites): `n = (int)f; if f != ceil(f) and ceil(f)-f <= rand() then n+1`,
   i.e. rounds up with probability frac(f). Java's equivalent.

## 2. Wire

- Entity: actor type **69 `xp_orb`** (DATA `protocol.json` entity types; BDSRE `0x45` in every create).
  Sent as `AddEntity` with `entity_type` `minecraft:xp_orb` (mem for the exact string).
- Value: entity data **key 15 `experience_value`** (`MetadataDictionaryItemKey::ExperienceValue`), int
  (DATA). BDSRE: every value write goes to data item index 15 (`items + 0x78`), type 2 (int), and marks it
  dirty, so a merge sends `SetEntityData` with the new value. The client sizes the orb from it (inf).
- Movement: orb `normalTick` returns right after `age++` when `isClientSide` (BDSRE 142f9d480), so the
  client does **not** home or simulate orbs; it only shows what the server moves. Motion packets: the
  generic actor sync (`MoveEntityDelta`, `SetEntityMotion` on motion change): mem, not traced here.
- Removal (pickup, age, merge): `Actor::remove` (vtable +0x58) -> `RemoveEntity` (mem for the packet).
- **No `TakeItemEntity`** for orbs: `playerTouch` never builds one (BDSRE).
- Save tags of an orb (BDSRE 142f9e100): `Age` (short), `experience value` (int, with the space).

## 3. Orb tick and pickup: `ExperienceOrb::normalTick` 142f9d480, `playerTouch` 142f9e210

1. **Despawn**: `age++`; remove when the old age > 5998, i.e. at age 6000 (BDSRE).
2. **Merge**, server, every 20 ticks (`tickCount % 20 == 1`): orbs in `[pos, pos+1]` whose **unique ids
   are congruent mod 13** (lambda 142fa1180) are absorbed: this.value += other.value (skip on i32
   overflow), this.age = min(ages), other removed. (Java 1.17+ uses id % 40.)
3. **Physics**: motion.y -= 0.04; in lava (block at floor(pos)): motion = ((r-r)*0.2, 0.2, (r-r)*0.2) and
   `LevelSoundEvent` 27 `Fizz`; `move(motion)`; friction 0.98 in air, on ground `blockFriction*0.98`
   (0.588 for normal blocks); motion.y *= 0.98, and on ground motion.y *= -0.5 (bounce).
4. **Target**: nearest player (passing a virtual filter, 14188aea0) with **distance^2 < 64** (8 blocks,
   from orb pos to player StateVector pos) (lambda 14188b1a0). Searched only when
   `tick - 20 - (uniqueId % 100) > lastSearch`, and then only if there is no target or it is > 8 away.
   First search is 21-120 ticks after spawn (lastSearch starts 0).
5. **Homing**: `d = (playerAttachPos(Body) - pos) / 8`; `h = sqrt(d.x^2 + d.z^2)` (**horizontal only**).
   - `h <= 0.225` (within 1.8 blocks across): move by `d` directly; if the player's flag at +0x210 bit 0
     is set, call `playerTouch(player)`.
   - else `f = 1 - h`; if f > 0: `motion += d / h * f^2 * 0.1` (all three axes).
6. **playerTouch**: server only; player vtable +0x170 check (likely alive); if `mRandomPickupValue`
   (+0x3c4) != 0, pick up only when `random.nextInt(it) == 0`; fires `PlayerGetExperienceOrbEvent`
   (cancellable). Then `LevelEvent` **1051 `SoundOrb`** at the **orb** position, data 0 (BDSRE 0x41b).
   Mending: one random equipped Mending item, repair = min(damage, value*2), value -= repair/2 (just note).
   Remaining value > 0 -> `Player::addExperience(value)` (vtable +0x6b8). Orb removed.
7. No Java-style `takeXpDelay` in playerTouch; several orbs can be absorbed in one tick (inf).

## 4. Player XP: `Player::addExperience` 1402c5190, `Player::addLevels` 1402c5750

- Same function in Player (14a67c130) and ServerPlayer (14a70c930) vtables, slots 0x6b8 / 0x6c0 (BDSRE).
- **xpNeeded(L)**: `L/15 == 0 -> 2L+7; == 1 -> 5L-38; else 9L-158` (L<15, 15..29, >=30). Same as Java.
  Cached at +0xb24, dirty flag +0xb23.
- **addExperience(n)**: totalXp (+0x488) += n; `progress += n / need` (float div); while progress >= 1:
  `addLevels(1)`, `progress = (progress - 1) * oldNeed / newNeed`; then set `player.experience` = progress.
- **addLevels(n)**: if level+n < 0: level = 0, progress = 0. Else level += n. Then if n > 0, newLevel % 5 == 0
  and > 100 ticks since the last one: `LevelSoundEvent` **62 `levelup`** (string table 14346a2b0) at the
  player's Body attach pos, `extra_data = level > 30 ? 0xBFFFFF : (int)(level/30 * 0.75 * 0xFFFFFF)`
  (a volume, 0.75 max). No `ActorEvent` 34 `player_add_xp_levels` from here (BDSRE).
- Attributes go out in `UpdateAttributes` when dirty (mem). Recorded join values (REC,
  `sapling-data/data/UpdateAttributes.bin`, field order min,max,current,default_min,default_max,default):
  - `minecraft:player.level`: 0, **24791**, 0, 0, 24791, 0
  - `minecraft:player.experience`: 0, 1, 0, 0, 1, 0 (progress 0..1)
  - acacia-bot reads both (`acacia-bot/src/state/player.rs:216`).
- No `SetPlayerXp` packet exists in protocol 2193; level/progress travel only as attributes (DATA).

## 5. Save (player NBT, BDSRE)

- Write 1402b2d30: **`PlayerLevel` Int** = `(int)` truncation of the level attribute; **`PlayerLevelProgress`
  Float** = the experience attribute.
- Read 1402af220: both keys must exist, else neither is applied; `PlayerLevel` Int -> float attribute,
  `PlayerLevelProgress` Float. **`EnchantmentSeed` Int** (+0xb70).
- Furnace (1457f3ed0 read / 1457f4220 write): `StoredXPInt` Int in hundredths of XP; legacy `StoredXP` Short.

## 6. Furnace XP (brief; furnaces doc has the rest)

- Per-item XP from 1457f60a0 (one item hard-coded 0.35; the rest via the item/recipe table, not read here).
- Output take: `randomRound(count * xpPerItem)` per take + `randomRound(StoredXPInt/100)`; both to the
  player as orbs at the player (section 1.7).

## sapling mapping

- `mobs::spawn_item` / `drops.rs` items never move and are announced once; orbs need: an `AddEntity`
  helper (type `minecraft:xp_orb`, metadata key 15), per-tick physics + homing (section 3), movement
  packets, `RemoveEntity` on pickup, `LevelEvent` 1051, and `UpdateAttributes` for level/experience.

## Not settled

1. The exact movement packets BDS sends for a moving orb (`MoveEntityDelta` flags, any `SetEntityMotion`).
2. Whether the player-side entity touch loop also calls `playerTouch` (pickup without homing), and what
   the player flag at +0x210 bit 0 is.
3. What filter 14188aea0 applies (spectators / dead players excluded?).
4. Player StateVector pos = eye height (feet+1.62) and the Body attach-pos offset: mem, not read.
5. Exact `AddEntity` identifier string and orb collision size (0.25? mem).
6. Whether `UpdateAttributes` for xp goes only to the player itself.
