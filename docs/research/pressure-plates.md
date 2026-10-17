# Bedrock pressure plates, from BDS (bdsre)

Read 2026-10-06 from `acacia/.testserver/bds-molang/bedrock_server.exe` (PE stamp 6ab54bee; the default
`.testserver/bds/` path no longer exists, so `BDS_EXE` must point here) with `py -3 bdsre.py c|dis|win <va>` and
the LeviLamina headers `BasePressurePlateBlock.h`, `PressurePlateBlock.h`, `WeightedPressurePlateBlock.h`.
Entry point: the RTTI of `subscribe<BasePressurePlateBlock, BlockQueuedTickEvent>`
(`find PressurePlate`) → its three constructors → the tick member pointer and the two vtables. No `name` labels
added; suggested labels below. The Java cross-check sources (`research/java-truth`) were not on disk; Java
statements here are from memory and marked so.

| VA | Suggested label | What |
|---|---|---|
| 14879da90 | `PressurePlateBlock::PressurePlateBlock` | stores `mSensitivity` at `+0x398`, subscribes tick |
| 148a9fab0 | `PressurePlateBlock::PressurePlateBlock` (archetype) | same vtable; data-driven `pressure_plate_block` |
| 1487a7c20 | `WeightedPressurePlateBlock::WeightedPressurePlateBlock` | stores `mMaxWeight` at `+0x398` |
| 14a84e050 / 14a784470 | `PressurePlateBlock::vftable` / `WeightedPressurePlateBlock::vftable` | |
| 142cd0760 | `BasePressurePlateBlock::tick` | the queued tick |
| 142cd0be0 | `BasePressurePlateBlock::checkPressed` | set state, sound, game event, script event, reschedule |
| 142cd17f0 | `BasePressurePlateBlock::entityInside` | vt+0x418; the press |
| 142cd1770 | `BasePressurePlateBlock::shouldTriggerEntityInside` | vt+0x290; xp-orb and custom-component filter |
| 142cd1e00 | `BasePressurePlateBlock::getSensitiveAABB` | vt+0x440 |
| 142cd09a0 | `BasePressurePlateBlock::getVisualShape` | vt+0x50 |
| 142cd0b10 | `BasePressurePlateBlock::mayPlace` | vt+0x210; block below gives centre support |
| 142cd0bc0 | `BasePressurePlateBlock::isAttachedTo` | vt+0x280; pos below |
| 141a27610 | `BlockType::canSurvive` (shared) | vt+0x3c8; tail-calls vt+0x210 |
| 142cd0b00 | `BasePressurePlateBlock::getTickDelay` | `return 20` (vt+0x420) |
| 140214560 | (folded `return 10`) | Weighted `getTickDelay` |
| 144bbba90 / 141f0b420 | `PressurePlateBlock::getSignalStrength` / `WeightedPressurePlateBlock::getSignalStrength` | vt+0x428 |
| 144bbba80 | `PressurePlateBlock::getSignalForData` | vt+0x430: `d == 1 ? 15 : 0` (Weighted: identity 1412b4a10) |
| 144bbba70 | `PressurePlateBlock::getRedstoneSignal` | vt+0x438: `s > 0 ? 1 : 0` (Weighted: identity) |
| 144bbbd50 / 141f0b500 | `PressurePlateBlock::_addHardCodedBlockComponents` / Weighted | vt+0x390, per-state redstone power (inferred) |
| 1415f5520 | `BlockTickingQueue::removeTick?` | marks queued ticks at pos of this block type removed |
| 1415f50f0 | `BlockTickingQueue::add?` | |

## Constants (verified on BDS)

| | wood plates | stone, polished_blackstone | light_weighted | heavy_weighted |
|---|---|---|---|---|
| sensitivity | 0 Everything | 1 Mobs (`ActorType` mask 0x100) | count | count |
| max weight | | | 15 | 150 |
| tick delay while pressed | 20 | 20 | 10 | 10 |
| `redstone_signal` when pressed | 1 | 1 | signal 1..15 | signal 1..15 |
| redstone power output | 15 | 15 | = `redstone_signal` | = `redstone_signal` |

Evidence: per-plate arguments at the 15 `14879da90` call sites in `14849c7e0` (legacy id 0x46 stone sens 1,
0x226 polished_blackstone sens 1 via `[rbp+0x1810]=1`, 0x48 wooden and the other wood ids sens 0) and the two
`1487a7c20` sites (id 0x93 → 15, id 0x94 → 150). `Sensitivity::Players` (2, `ActorType` 0x13f) exists but no
vanilla plate uses it.

- Sensitive box (`142cd1e00`): `(x+1/16, y, z+1/16) .. (x+15/16, y+0.25, z+15/16)`.
- Visual shape (`142cd09a0`): `1/16 .. 15/16` across, height `1/16` up, `1/32` pressed (table 14a7d4c78).
- Weighted signal (`141f0b420`): `n = min(count, max)`; `n > 0 ? (int)ceilf(n / max * 15f) : 0`
  (light: `min(n,15)`; heavy: `ceil(min(n,150) / 10)`).
- Block names (registry `acacia-world/data/blocks.bin`, all with `redstone_signal=0..15`):
  `minecraft:{stone, polished_blackstone, wooden, spruce, birch, jungle, acacia, dark_oak, mangrove, cherry,
  bamboo, pale_oak, crimson, warped, poplar, light_weighted, heavy_weighted}_pressure_plate`. `wooden_` is oak.

## Pseudo-code (verified on BDS unless marked)

```
redstone(plate)  = plate.redstone_signal                    # the block state
old_signal(s)    = PLAIN ? (s > 0 ? 15 : 0) : s             # getSignalForData(getRedstoneSignal(s))
state_for(sig)   = PLAIN ? (sig > 0 ? 1 : 0) : sig          # getRedstoneSignal

signal(pos):                                                # getSignalStrength
    box = sensitive box of pos
    if WEIGHTED:
        n = count of actors in box that are not xp orbs     # spectators are NOT excluded here
        n = min(n, MAX); return n > 0 ? ceil(n / MAX * 15) : 0
    actors = Everything: all actors in box; Mobs: actors of type Mob (players are mobs)
    for a in actors, skipping xp orbs (ActorType 0x45):
        if a is not a player: return 15
        if game_type(a) == Spectator(6) or (== Default(5) and world default is Spectator): continue   # inferred
        return 15
    return 0

entity_inside(pos, actor):                                  # 142cd17f0, server only
    if actor is xp orb: return
    if actor+0x210 bit0 and !fcn.1402ada30(actor, 2): return  # not decoded, see Unverified
    old = old_signal(redstone(pos)); new = signal(pos)
    if new > 0 and old == 0:                                # fresh press
        remove queued ticks for pos
        check_pressed(pos, actor, 0, new)
    elif new > 0 and old != new:                            # current-version branch; only weighted can differ
        set(pos, plate[state_for(new)], flags 3)            # no sound, no reschedule

tick(pos):                                                  # 142cd0760, server only
    old = old_signal(redstone(pos)); new = signal(pos)
    check_pressed(pos, none, old, new)
    if !can_survive(pos): level.destroy_block(pos, drop)    # vt+0x3c8 → mayPlace; destroy call inferred (Level vt+0x7c8)

check_pressed(pos, source, old, new):                       # 142cd0be0
    set(pos, plate[state_for(new)], flags 3)                # always, even when unchanged
    if (old > 0) != (new > 0):
        sound = new > 0 ? PressurePlateClickOn(449) : PressurePlateClickOff(448)   # 73/74 on the old-version branch
        level_sound(sound, (x+0.5, y+0.1, z+0.5), extra = runtime id of the state just set)
        game_event(source, new > 0 ? block_activate : block_deactivate, pos)       # globals 14cc8a93c/14cc8a944, names inferred
    if new <= 0:
        if old > 0: script PressurePlatePopAfterEvent(pos, old, new)
        return                                              # no reschedule: plate is idle again
    if old != new: script PressurePlatePushAfterEvent(pos, old, new, source)
    schedule(pos, DELAY)                                    # 20 plain, 10 weighted, every tick it stays down
```

So a plate goes down the tick an entity's movement reaches `entity_inside` with something in the sensitive box,
and is re-checked every `DELAY` ticks while down; it comes up on the first check that finds the box empty (between
0 and DELAY-1 ticks after the last entity left, plus one DELAY minimum after the press). A weighted plate also
changes level between checks whenever an entity inside it ticks.

## Answers

1. Names: table above; one property `redstone_signal` (0..15) on all 17. Plain plates use 0/1 in that state
   (BDS 144bbba70), not 15; the 15 is the power they emit. High confidence on the decompile; not yet seen on a
   live BDS (see Unverified).
2. Wood: every actor but xp orbs (items, arrows, boats, minecarts, mobs, players). Stone and polished blackstone:
   mobs and players only. Spectators never press plain plates. Weighted: every actor but xp orbs, counted as
   above. Box: 1/16 inset per side, 0.25 high (Java, from memory: the same `TOUCH_AABB` 0.0625/0.25; the task's
   0.125 is not what BDS uses).
3. Press: in `entityInside`, the tick the entity overlaps the cell and the box has something in it. Then
   `checkPressed` schedules a tick every 20 (plain) / 10 (weighted) ticks; each tick recounts and releases on 0.
   Java (memory): plain `getPressedTime` 20, weighted 10, same shape.
4. Sounds: LevelSoundEvent `PressurePlateClickOn` 449 (`pressure_plate.click_on`) / `PressurePlateClickOff` 448
   (`pressure_plate.click_off`), position `(x+0.5, y+0.1, z+0.5)` (not the centre), extra data = runtime id
   (`Block+0x114`) of the state just written, so the client picks wood/stone/metal from it. There is no per-wood
   sound name in BDS: only those two strings exist (`str click_o`). In sapling's string form:
   `send::sound("pressure_plate.click_on", ...)` with the new state, then `to_viewers`, as `doors::play` does.
   Only on 0↔nonzero transitions; a weighted plate going 3→5 is silent.
5. `checkPressed` writes with `BlockSource::setBlock(pos, block, 3, null sync data)`: flags 3 = neighbours +
   network, and no actor is excluded, so the change goes out as a normal UpdateBlock to everyone who sees the
   chunk, the presser included. Nothing in the decompile suggests client prediction; send to all viewers.
6. Sapling: `movement::player_input` (`sapling-shard/src/movement.rs:159`, scheduled in `shard.rs:84`) writes
   `Position { feet, on_ground, .. }` (`components.rs:23`) from each PlayerAuthInput; players' world is
   `InLayer` (`components.rs:32`). A per-tick "who overlaps which plate" pass belongs in `handlers::dispatch`
   next to `blocks::release_buttons` (`handlers.rs:226`), which already has `&mut World` and `Tick`; the plate
   tick queue can mirror `doors::Pressed`. Player box: feet ± 0.3 horizontally, 1.8 high. Item drops
   (`drops.rs`) and mobs (`mobs::MobGrid`) are the other actors a wooden plate would count.

## Unverified / inferred

- `redstone_signal=1` for a pressed plain plate is read off `getRedstoneSignal` (144bbba70); confirm once by
  standing on a stone plate on BDS and reading the UpdateBlock.
- Spectator rule: `fcn.141cd3970(actor+8)` read as "game type, -1 for non-players" and Level vt+0x460 as the
  default game type, from the values 5/6 only.
- `entityInside`'s early return on `actor+0x210 bit0 && !fcn.1402ada30(actor, 2)`: not decoded (maybe a
  player-ability or "ignores block triggers" test).
- Which actors reach `entityInside` and with which box (the caller, actor movement, was not decompiled); the
  sensitive box decides the signal anyway, so overlap of the cell is enough to start the check.
- The old-version branch (globals 14cc48318/14cc48338 and a version compare against 14cc48300) uses
  PowerOn/PowerOff (73/74) and skips the weighted in-place update; not needed for a current client.
- `neighborChanged` (folded, not decompiled) presumably breaks the plate when `canSurvive` fails, as `tick` does.
- Game-event names for 14cc8a93c / 14cc8a944 inferred (block_activate / block_deactivate).
- Java 26.3 sources were not found under `research/java-truth`; every Java remark is from memory.
