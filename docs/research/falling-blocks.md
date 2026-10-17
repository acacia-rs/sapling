# Bedrock falling blocks, from BDS (bdsre)

Read 2026-10-06 from `acacia/.testserver/bds-molang/bedrock_server.exe` (newest packs `vanilla_1.26.50`; the
default `bds/` folder no longer exists, so every command ran with `BDS_EXE` pointing here and `build` was run once).
Pseudo-C came from `py -3 bdsre.py c <va>`, virtual slots were checked with `dis`. Slot names come from the
LeviLamina headers (`BlockType.h`, `FallingBlock.h`, `FallingBlockActor.h`,
`falling_block_system_on_land/*.h`). The `BlockType` slots match the binary exactly here: `FallingBlock` adds
0x420..0x448 straight after `BlockType`'s last slot. Block name globals were resolved from their initialisers in
`fcn.1425f41e0`. No `name` labels were added; suggested labels are below.

No recording anywhere in the workspace has a falling block in it. The bot traces (`acacia-bot/tests/traces/*.gz`)
have none, and the realm capture (`research/mitm-realms/capture/*.jsonl`) has the string only in
AvailableEntityIdentifiers and AvailableCommands. `research/java-truth` does not exist on this machine, so nothing
was cross-checked against Java. The "Java" notes below are from memory and marked as such.

| VA | Suggested label | What |
|---|---|---|
| 14a8e44a0 | `FallingBlock::vftable` | every falling block ctor stores it first |
| 14714f5b0 | `FallingBlock::onPlaceBase` | queue a tick, delay 2 |
| 147150750 | `FallingBlock::neighborChanged` | queue a tick, delay 1 if the block below changed, else 2 |
| 14714fa10 | `FallingBlock::tick` | server only, calls `checkSlide` |
| 147150590 | `FallingBlock::checkSlide` | walks down a stack, then `isFreeToFall` and `startFalling` |
| 147150300 | `FallingBlock::isFreeToFall` | the "can fall into the cell below" test |
| 14714fed0 | `FallingBlock::startFalling` | spawns the actor, sets the cell to air |
| 14714fa70 | `FallingBlock::_tickBlocksAround2D` | ticks the falling blocks beside the cells above and below |
| 141a21740 | `BlockType::canSlide` | `BlockType+0x163 & 4` |
| 141a28be0 | `BlockType::shouldStopFalling` | default landing test (OnGround) |
| 141a27630 | `BlockType::tryToPlace` | `mayPlace`, then `setBlock(.., 3)` |
| 142ceddd0 | `Block::breaksFallingBlocks` | wraps `BlockType` vt+0x238 |
| 145984830 | `FallingBlockActor::doNormalTick` | body of `FallingBlockNormalTickSystem::fn0` (14469ee50) |
| 145985930 | `FallingBlockActor::setFallingBlock` | writes the `Variant` data item, `mHurtEntities`, `mCreative` |
| 145983f70 | `FallingBlockActor::breakBlock` | drops the block as an item, particles, removes the actor |
| 145985820 | `FallingBlockActor::_isBeingPushedUp` | piston pushing up from below |
| 1446c4fd0 | `FallingBlockNormalTickSystem::tick` (already labelled) | runs the deferred `onLand`/`breakBlock` queue after the view |
| 14a836500, 1446a6100, 1446a61b0 | `FallingBlockSystemOnLand::SystemTickDelegate::{vftable,onLand,breakBlock}` | queue only |
| 141a2d0e0 / 14a7649f0 / 141a2dc50 | `BrushableBlock::{ctor,vftable,onLand}` | suspicious sand/gravel (ctor compares the name to `suspicious_gravel`) |
| 14580eca0 / 14a87b800 / 14580f3b0 | `AnvilBlock::{ctor,vftable,onLand}` | `falling()` true; LevelEvent 1022 |
| 148a2a460 / 14a93ed50 | `DragonEggBlock::{ctor,vftable}` | no falling overrides |
| 148b35ff0 / 14a949850 | `ConcretePowderBlock::{ctor,vftable}` | |
| 148b36450, 148b368e0 | `ConcretePowderBlock::{onPlace,tick}` | harden first, else the FallingBlock path |
| 148b37310 | `ConcretePowderBlock::_touchesWater?` | Water material at up, -z, +z, -x, +x |
| 148b36980 | `ConcretePowderBlock::_tryHarden?` | powder to concrete name map, then set |
| 148b37770 | `ConcretePowderBlock::shouldStopFalling` | hardens in a water cell mid-flight |
| 1471fa890 / 14a8ef730 | `TopSnowBlock::{ctor,vftable}` | clears `0x163` bits 0x04 and 0x80 |
| 1471fdbd0, 1471fdcb0, 1471fdaa0 | `TopSnowBlock::{isFreeToFall,startFalling,shouldStopFalling}` | |
| 14660a5a0 / 14a8bb620 / 14660dd80 / 14660e100 | `PointedDripstoneBlock?::{ctor,vftable,onLand,startFalling}` | inferred: `falling()` true and its own startFalling |
| 148b3b710 / 14a94acf0 | `ColoredFallingBlock?` (sand, gravel) | no overrides of the falling slots |
| 148b3df00 / 14a94b650 | unidentified falling block | own neighborChanged 148b3fff0 |
| 140d50930 | `Actor::sendMotionPacketIfNeeded?` | called by `ActorMotionSyncSystem::tick` (145e2d9f0) for every actor |
| globals | 14ccdb710 `fire`, 14ccd33b8 `soul_fire`, 14ccdb870 `snow_layer`, 14ccda3a8 `suspicious_gravel`, 14ccc5440 `snowball` | HashedString, hash at +0 and string at +8; `air` 14ccf9950 per water.md |
| coral lists | 14ccd9440 (10 coral blocks, live and dead), 14ccd9458 / 14ccd3238 coral fans (live / dead), 14ccd3250 / 14ccd3268 wall fans (live / dead) | vectors of HashedString pointers |

## Constants (verified on BDS)

| | value | evidence |
|---|---|---|
| tick after the block is placed | 2 | 14714f5b0 `schedule(.., 2, 0)` (also concrete powder 148b36450) |
| tick after a neighbour changes | 1 if it is the block directly below, else 2 | 147150750 |
| ticks for side neighbours after a fall starts | 2 | 14714fa70 |
| gravity, then drag | `vy -= 0.04`; move; `v *= 0.98` (all three axes) | 145984a43..a81 |
| landing bounce | `vx, vz *= 0.7`, `vy *= -0.5` | 14598507d |
| airborne timeout | `mTime > 600`: `breakBlock` (drops the item) | 145984830 |
| out of the world | `mTime >= 101` and `y < dimension min height`: removed, no drop | 14598531b..350 |
| removal after placing | `WaitRemoval`; removed on the 3rd tick after landing (`mWaitTicks` 0, 1, 2, then remove) | 145984a02 |
| actor type | 0x42 (66) `minecraft:falling_block` | 14714fed0 `fcn.141ce0430(.., 0x42, ..)` |
| spawn position | `(x + 0.5, y + 0.5, z + 0.5)`, velocity 0 | 14714fed0 |
| block carried in | data item 2 (`Variant`), type Int, = `Block+0x114` (`mNetworkId`) | 145985930 |

`FallingBlockActor` fields (LeviLamina order, offsets confirmed by use): `mCreative` 0x3b0, `mState` 0x3b4,
`mWaitTicks` 0x3b8, `mTime` 0x3bc, `mCancelDrop` 0x3c0, `mHurtEntities` 0x3c1, `mFallDamageMax` 0x3c4,
`mFallDamageAmount` 0x3c8, `mFallingBlockSerId` 0x3d8, `mFallingBlockId` 0x3e0.

## Pseudo-code (verified on BDS unless marked)

```
on_place(p):        if not instaticking(p): schedule(p, 2)                       # 14714f5b0
neighbor_changed(p, n):                                                          # 147150750
    schedule(p, n == p.down ? 1 : 2)
tick(p): if server: check_slide(p)                                               # 14714fa10

check_slide(p):                                                                  # 147150590
    q = p
    while q.y - 1 > min_y and block(q.down).type.canSlide(): q = q.down          # canSlide = BlockType+0x163 & 4,
    if q.y > min_y and type(p).isFreeToFall(q):                                  #   set by every FallingBlock ctor,
        start_falling(q, block(q), creative)                                     #   cleared by TopSnowBlock
    # the LOWEST block of a falling-block stack falls first; the next one follows on its own
    # neighbor_changed (delay 1), so a column comes down one block per tick from the bottom

is_free_to_fall(p):                                                              # 147150300
    b = block(p.down)
    return b is air | fire | soul_fire | snow_layer (any height)
        or material(b) in {Air, Water, Lava, Bubble, Plant, StructureVoid}
        or b is any coral, coral fan or coral wall fan (live or dead)

start_falling(p, block, creative):                                               # 14714fed0
    a = spawn actor type 0x42 at (p + 0.5 on all three axes)
    a.set_falling_block(block, creative)     # Variant(2) := block network id; mHurtEntities := type.falling()
    set(p, air, flags 3)                                                          # neighbours + network
    for c in [p.up, p.down]: for n in 4 sides of c:
        if block(n).type.canSlide(): schedule(n, 2)                               # 14714fa70
    level.add_entity(a)

actor_tick(a):                                                                   # 145984830, server only
    if a.state == WaitRemoval: if a.wait++ >= 2: remove(a); return
    a.prev = a.pos; a.time += 1
    a.vel.y -= 0.04; a.move(a.vel); a.vel *= 0.98
    c = floor(a.pos); blk = block_by_network_id(a.Variant)
    if not a.HAS_COLLISION and blk's box at a.pos overlaps no collision box: set HAS_COLLISION (metadata flag 48)
    breaker = an entity in a's AABB has BreaksFallingBlocksFlagComponent
    landed = breaker or blk.type.shouldStopFalling(a)
             or (blk.mLavaFlammable(+0xa6) == 1 and material(block(c.down)).mSuperHot)   # offsets inferred
    if not landed:
        if a.time > 600: break_block(a)                                           # deferred, see below
        elif a.time > 100 and a.pos.y < min_y: remove(a)
        return
    a.vel.x *= 0.7; a.vel.z *= 0.7; a.vel.y *= -0.5
    if a.cancel_drop: remove(a); return
    breaks = false
    if blk.type+0x163 & 0x80:                                                     # cleared for snow layers
        breaks = block(c).breaksFallingBlocks() or block(c.down).breaksFallingBlocks()
                 or breaker or (an entity in cell c.down, not ActorType 90 (boat?), has BreaksFallingBlocks)
    if not breaks:
        placed = blk.type.tryToPlace(c, blk)         # mayPlace(default state, c) then set(c, blk, flags 3)
        if not placed and blk is a FallingBlock and block(c.down).type+0x132 & 0x80:
            set(c, blk, flags 3); placed = true                                   # bit not identified
        if placed:
            queue on_land(blk.type, c)                                            # runs after the system loop
            a.state = WaitRemoval; a.wait = 0; a.vel.x = a.vel.z = 0; return
    if not being_pushed_up_by_piston(c): queue break_block(a)

shouldStopFalling default (141a28be0): has OnGroundFlagComponent and (a block search with argument 0x13 in the
    AABB widened 1 down and 1 up finds nothing, or y <= min_y - 40). The 0x13 search was not resolved.

break_block(a):                                                                  # 145983f70
    b = block_by_network_id(a.Variant)
    if b is not air and actor vt+0x3d0 (shouldDropDeathLoot?):
        drop = b is snow_layer ? 1 snowball : b.type.asItemInstance(b)            # the block itself, NOT its loot table
        spawn the item at a's position
    LevelEvent ParticleDestroy (2001) at a.pos, data = b
    remove(a)
```

On-land hooks, run by `FallingBlockNormalTickSystem::tick` after every actor has ticked:

- Default (sand, gravel, concrete powder, dragon egg, snow): none (`140075fb0` is `ret`).
- Anvil (14580f3b0): LevelEvent 1022 `SoundAnvilFall` at the landing cell. The data looks like 0. Whether the
  position is the cell corner or centre was not settled.
- Suspicious sand and gravel (141a2dc50): LevelEvent 2001 `ParticleDestroy` at the cell, data = the default
  state's id, then BlockSource vt+0x110 on the cell. That call is inferred to remove the block, so these break on
  landing and drop nothing.
- Pointed dripstone (14660dd80): not decompiled.

Concrete powder:

- Place and tick (148b36450, 148b368e0) first try to harden. The powder turns into the matching
  `<colour>_concrete` when any of up, -z, +z, -x or +x has Water material; down does not count. If it hardens,
  nothing is queued.
- Mid-flight (148b37770): when the actor's own cell has Water material, it runs the same hardening at that cell.
  If that converts, the actor is removed at once and no landing happens. Otherwise the default
  `shouldStopFalling` applies. Checked on BDS 2026-10-08 (`bds_command_probe concrete`,
  research/captures/bds-concrete.txt): the cell is the one holding the box's middle, and "the same hardening"
  needs water above or beside that cell. Into a pool two deep the powder became concrete in the bottom water cell
  (plain UpdateBlock, RemoveEntity at once); into a pool one deep it landed as powder.

Snow layer (`TopSnowBlock`):

- It falls, but `canSlide` is off, so it never takes part in the stack walk. The `breaksFallingBlocks` checks are
  off too.
- Its `isFreeToFall` adds Leaves material below as "free". This may be the inverse of what was meant; it is
  listed under Unverified.
- `shouldStopFalling` also stops over Water material.
- `startFalling` keeps or restores a covered extra-layer block. This was only skimmed.
- When it breaks, it drops 1 snowball.

## Answers

1. **When it starts.** `onPlace` queues a tick 2 ticks later. `neighborChanged` queues one 1 tick later for the
   block below, 2 for any other neighbour. The tick runs `checkSlide`, which needs `y > min_y` and
   `isFreeToFall(below)`. "Free" means:
   - air, fire, soul fire, or a snow layer of any height;
   - Water, Lava, Bubble, Plant or StructureVoid material;
   - any coral, coral fan or coral wall fan.

   Plant material probably covers the grasses and flowers. Which blocks have it was not dumped.
   (Confidence: high for the code; the Plant member list is unknown.)
2. **The start.** `set(p, air, flags 3)` sends UpdateBlock. Then the actor `minecraft:falling_block`
   (type 0x42) spawns at `(x+.5, y+.5, z+.5)` with zero velocity. Data item 2 (`Variant`, Int) carries
   `Block::mNetworkId`, which is the same id BDS puts in UpdateBlock and LevelSoundEvent. In sapling that is
   `registry.network_hash(id)`, which `send::destroy_particles` already uses. `HAS_COLLISION` (flag bit 48) is
   added on the first tick the block's box overlaps no collision box. (Confidence: high.)
3. **The actor tick.** Gravity 0.04, then drag 0.98. Landing happens when it is on the ground, a block or entity
   breaks it, or a lava-flammable block falls onto lava.
   - It places itself in `floor(pos)` through `tryToPlace` (`mayPlace` of the cell, i.e. the cell must give way).
   - If the cell refuses, or the cell or the block below `breaksFallingBlocks` (torches, slabs and so on, per
     block), it drops **itself as an item** (`asItemInstance`, not the loot table), shows destroy particles and
     goes. A piston pushing up from below is the exception.
   - Airborne for more than 600 ticks: dropped the same way. Below the world after 100 ticks: removed without a
     drop.
   - Anvil: LevelEvent 1022 on landing. Concrete powder hardens next to water, or in it mid-flight.
   - Anvil and dripstone set `mHurtEntities` (fall damage) through `falling()`; damage was not decoded.
4. **Movement.** Nothing specific to falling blocks; there is no recording. The actor goes through the generic
   per-actor sync: `ActorMotionSyncSystem::tick` calls `sendMotionPacketIfNeeded` (140d50930) for each actor,
   which sends SetActorMotion when the motion changed, and the position goes out through the usual actor-moved
   path (MoveActorDelta, the same as mobs; 3362 of them in the realm capture). Since the velocity changes every
   tick, expect a SetActorMotion and a MoveActorDelta roughly every tick while it falls. (Confidence: medium, not
   recorded.)
5. **Landing order.** The UpdateBlock for the placed block comes on the landing tick, then the on-land hook
   (anvil sound, or particles plus air for suspicious blocks) later in the same tick. RemoveActor comes 3 ticks
   later. For a refused landing, on the same tick: the item entity (AddItemEntity), LevelEvent 2001 at the
   actor, then RemoveActor.
6. **Sapling.** See the next section.

## What sapling needs (6)

Today `updates.rs::fall` writes air and then the block in the lowest free cell, in the same step.

`sapling_data::add_entity` with `identifier: "minecraft:falling_block"` already does most of the AddActor:

- `variant: registry.network_hash(id) as i32` writes `Variant` (key 2, Int), which is the only item BDS sets from
  the block.
- `falling_block` is not in `Mobs.bin`, so the packet comes from `plain()`. That gives flags `HAS_COLLISION |
  AFFECTED_BY_GRAVITY`, plus `Color`, `Nametag`, `Scale` 1 and `MarkVariant`, which are harmless.

What is missing or wrong:

- **Hitbox.** `plain()` uses 0.9 x 0.9. A falling block is 0.98 x 0.98 (Java; DF and PM use the same). Pass the
  size in, or record one BDS falling block into `Mobs.bin` and drop the guesswork.
- **Position.** `feet` goes onto the wire unchanged. Send the cell centre at spawn, `(x+0.5, y+0.5, z+0.5)`, as BDS
  does. If sapling keeps feet internally, the network position is about `feet + 0.49` (DF/Nukkit base offset,
  unverified on BDS).
- **Flags.** BDS adds `HAS_COLLISION` a tick later. Sending it from the start is fine for a client.
- **Movement packets.** sapling has none. Nothing in `sapling-shard` or `sapling-data` builds MoveActorDelta,
  MoveActorAbsolute or SetActorMotion, and `mobs.rs` treats every actor as a statue: an announcement made once and
  replayed to new viewers. A falling block needs a per-tick entity:
  - simulate it on the server (the three lines above);
  - send each viewer a MoveActorDelta (or MoveActorAbsolute) of the new position and a SetActorMotion of the
    velocity every tick;
  - when it lands, the UpdateBlock and on-land packets, then RemoveActor 3 ticks later. Removal can reuse
    `mobs::remove`.

  It also needs a current announcement for late viewers (the position at the time they see it), not the spawn
  one frozen in `Mob::announcement`.
- **Refused landing drops.** Use the block's own item (`asItemInstance`; gravel drops gravel, never flint), not
  `loot::thrown`. Snow layers drop 1 snowball. Use `mobs::spawn_item` at the actor position, plus
  `send::destroy_particles` with the block's id.

## Differences from sapling's current rules

- `gives_way` decides where sapling's blocks stop. BDS uses `isFreeToFall` only to *start* a fall; after that,
  physics and `tryToPlace` decide. The start test includes snow layers of any height, corals and coral fans, and
  Plant material. It does not include generic "replaceable".
- Stack order: the bottom block falls first, then each next one a tick later. sapling's queue drops a whole tower
  in one step.
- Snow layers fall but do not slide or break on torches. Suspicious sand and gravel break on landing with no drop.

## Unverified / inferred

- How the generic sync handles falling blocks on the wire: MoveActorDelta and SetActorMotion every tick, or a
  client-simulated fall from one SetActorMotion. Also whether UpdateBlock(air) goes out before or after AddActor
  in the start tick. **One BDS capture settles both**: a bot places sand over a 3-block drop and the proxy records
  it.
- The actor's height offset (0.49 from DF/Nukkit), and so whether the wire y during flight is feet + 0.49.
- `Block+0xa6` = `mLavaFlammable` and `Material+6` = `mSuperHot` come from header offsets. `BlockType+0x132 & 0x80`
  (the forced place) and `0x163 & 0x80` (the break checks, on by default and cleared by TopSnow) are unnamed.
- The default `shouldStopFalling` block search with argument 0x13, and `fcn.140d27800(entity, 0x5a)` being
  "is a boat (ActorType 90)".
- `Actor` vt+0x3d0 (`shouldDropDeathLoot?`, one slot off the header) gating the drop. BlockSource vt+0x110 in the
  suspicious-block `onLand` being "remove block". `mCreative` comes from a level lookup (`0x14cc39898`) that was
  not resolved.
- Which blocks have Plant material, and which have `breaksFallingBlocks` true. Both are per-block data and were
  not dumped.
- The anvil LevelEvent position (cell corner or centre). Pointed dripstone (`startFalling` 14660e100,
  `onLand` 14660dd80), the 148b3df00 class, and anvil and dripstone fall damage were not decompiled.
- The TopSnow `isFreeToFall` Leaves-below rule as read, and its covered-block handling.
