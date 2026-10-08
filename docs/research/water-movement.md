# Water state and movement in water (BDS, bdsre)

Read 2026-10-08 from `acacia/.testserver/bds-molang/bedrock_server.exe` (PE stamp 6ab54bee) with a scratchpad copy
of bdsre (`c`, `dis`, `win`, `vt`, `rtti`, `str`, `imm`). Layouts are from the LeviLamina headers
(`UpdateWaterStateRequestComponent`, `OffsetsComponent`, `NavigationComponent`, `Material`, `IConstBlockSource`),
and offsets were confirmed by use. No labels were added. Nothing was tested on a live server. Tags:
**[V]** = read in code, **[I]** = inferred.

| VA | Suggested label | What |
|---|---|---|
| 142c2bed0 | `UpdateWaterStateRequestSystem::tick` (labelled) | resets the request (material Air, flags 0) for every entity with `InterpolateMovementNeededComponent` |
| 14661af60 | `LiquidPhysicsSystem::_liquidBlockFetch` | builds the boxes, sets `mMaterialType` and `mIsAnyWaterInBB` |
| 146621b20 | `LiquidPhysics::_anyMaterialInBox?` | material test per cell, optional `LiquidBlockEntry` list for flow |
| 1484043d0 | `InWaterSensingSystem::fn0` (labelled) | `mMaterialType == Water` -> `WasInWaterFlag` (see `splash-sound.md`) |
| 145ed7240 | `UnderWaterSensingSystem::doUnderWaterSensing` (labelled fn0) | sets `ActorHeadInWaterFlag` / `ActorHeadWasInWaterFlag` |
| 142f50c80 | `GetAttachPositionUtility::interpolatedPos?` | `prev + (pos - prev) * alpha`, rider and parrot cases |
| 142f503f0 / 142f50120 | `GetAttachPositionUtility::{getAttachPosition, _getBaseAttachPoint}` | switch over `ActorLocation` |
| 142ed6bc0 | `Block::getLiquidSurfaceHeight?` | `y + 1 - h`, see 1 |
| 141cd3240 | `OffsetsComponent::setBreathingOffset?` | writes `OffsetsComponent+0x38` |
| 145eed380 | `TravelTypeSensingSystem::fn0` (labelled) | picks Water / Lava / Ground / Air / Gliding travel |
| 141c59d00 | `register MobMovementDrag::{Water,Lava}` | strings `MobMovementDrag::Water`, `::Lava` |
| 141c67400 | `WaterDrag::apply` (labelled) | the water drag |
| 141c69960 | `MobMovementDrag::Lava` tick | `v *= 0.5` |
| 141c5ca50 | `register *Gravity` systems | `MobGroundGravity`, `MobAirGravity`, `LavaWalkAndSinkGravity`, `MobLavaGravity`, `MobWaterGravity`, `PlayerWaterGravity`, `ApplyGravity`, ... |
| 141c5c370 | `MobWaterGravity` per entity | `ApplyGravity = -0.02` |
| 141c5c610 | `MobLavaGravity` per entity | `ApplyGravity = -0.02` |
| 141c5c910 | `LavaWalkAndSinkGravity` per entity | -0.08 / -0.01 (lava walkers) |
| 141c867b0 | `ApplyGravity` per entity | `v.y += ApplyGravity.value` |
| 14845b3c0 | `WaterMoveSystem::fn0` (labelled) | moveRelative: `v += rotate(input) * speed / max(1, |input|)` |
| 148456490 | `WaterTravelSystem::fn0` (labelled) | the in-water travel speed (depth strider, `underwater_movement`) |
| 1424e5cb0 | `ClimbOutOfLiquidSystem::fn0` (labelled) | `v.y = 0.3` when the offset box is free |

## 1. The water state, each tick

Order: in the registration function 1418f9a60 these systems are registered one after another
(**[V]** order of registration; **[I]** that they run in that order, and before the AI step and
`ActorPostAiStepTickSystem`):

`SetPreviousPosRot` (prev := pos) -> `UpdateWaterStateRequest` (reset) -> `LiquidPhysics` (box tests) -> `InLavaSensing`
-> `InWaterSensing` (WasInWater, splash) -> `UnderWaterSensing` (head) -> `LiquidSplashRequest` ->
`LavaResetFallDistance` -> `LiquidSplash`.

Superseded by section 6: registration order is not execution order, and a live capture shows mob travel reading
the state one tick late.

### Box tests (`14661af60`, `146621b20`) [V]

```
box(a) = a.aabb                                  # (min, max), min = feet
if a is an item (ItemActorFlagComponent):
    lava_box = water_box = box(a)                # no shrink at all
else:
    lava_box  = shrink(box, x 0.1,   y 0.4,   z 0.1)     # each side
    water_box = shrink(box, x 0.001, y 0.401, z 0.001)
    # per axis: if min+d > max-d, both ends become the box centre on that axis
any_material(m, b):
    for every cell c with floor(b.min) <= c <= floor(b.max) (all three axes):
        if getLiquidBlock(c).material.type == m: return true      # IConstBlockSource vt+0x28
    return false                                                    # NO surface-height test
lava  = any_material(Lava 6,  lava_box)
water = any_material(Water 5, water_box)
request.mIsAnyWaterInBB = water
request.mMaterialType   = lava ? Lava : water ? Water : Air          # lava wins
```

- `getLiquidBlock` (vt+0x28, from the header order of `IConstBlockSource`) returns the liquid layer, so a
  waterlogged block counts as water. **[I]** from the vtable order only.
- Water material = `water`, `flowing_water`. `bubble_column` has its own material (Bubble), so it does **not**
  count. **[I]** from the material enum; the block -> material table was not dumped.
- Unlike Java, there is **no** liquid-height test here: a 1-high flowing water block counts over its whole cell.
- `InWaterSensing` then gives `WasInWaterFlag` iff `mMaterialType == Water` and the entity is not riding a boat.
  So an entity touching lava is never "in water" in the same tick. **[V]** (1484043d0, see `splash-sound.md`).

### Head in water (`UnderWaterSensingSystem`, 145ed7240) [V]

Applies to every entity with a `StateVector` and an `UpdateWaterStateRequest` (the view has no exclusions), so
items too.

```
head_was_in_water := head_in_water                                 # copy of the previous value
head_in_water = false
if request.mIsAnyWaterInBB:
    p  = prev + (pos - prev) * 0.0                                  # = pos, prev was just set to pos
    yb = p.y + Offsets.mBreathingOffset.y  (+ rider seat offset, - VanillaOffset; 0 for a plain actor)
    c  = (floor(pos.x), floor(yb), floor(pos.z))                    # x, z from the current pos
    L  = getLiquidBlock(c)
    if L.material.mLiquid and L.material.type == Water:
        d = L.liquid_depth (absent: treated as 0)
        h = d < 8 ? d / 9 : 0                                       # 142ed6bc0: (d<8 ? d+1 : 1)/9 - 1/9
        surface = c.y + 1 - h
        head_in_water = surface > yb                                # strict
```

Surface heights: source (0) and falling water (8..15) fill the whole cell (`y + 1`); depth 1 is `y + 8/9`, depth 7
is `y + 2/9`. Java uses `y + 1 - (d+1)/9` (a source is `y + 8/9`); BDS is 1/9 higher.

The `ActorLocation` is 7 = BreathingPoint (`getAttachPosition(.., 7, ..)`), `OffsetsComponent+0x38`.

### Formula for an item (box 0.25, pos = box centre = feet + 0.125)

- `ItemActor` ctor sets the breathing offset to `(0, 0.25, 0)` (145988f9f: `setBreathingOffset({0, 0.25, 0})`). **[V]**
- So `yb = pos.y + 0.25 = feet + 0.375`, i.e. 0.125 **above the top** of the item's box.

```
water_in_box(item) = any cell in floor(feet - (0.125, 0, 0.125)) .. floor(feet + (0.125, 0.25, 0.125))
                     whose liquid block is water / flowing_water
head_in_water(item) = water_in_box
                      and liquid block at (floor(x), floor(feet + 0.375), floor(z)) is water / flowing_water
                      and cell_y + 1 - (liquid_depth < 8 ? liquid_depth / 9 : 0) > feet + 0.375
```

`ItemActor::postNormalTick` (items.md) branches on `head_in_water`; it floats (vy eases to +0.06) only while that
point is under the surface, so an item settles with its breathing point at the surface: the box top about 0.125
under it. That last sentence is **[I]** (not observed).

For mobs the breathing offset comes from `AABBRelativeSizeUpdateComponent` (`y = box height * fraction`,
141cd3570) or the entity definition; the per-mob values were not read. **[I]**

## 2. Mobs without AI in water

Travel type (145eed380, mobs only: the view includes `MobTravelComponent`) **[V]**:

```
if PlayerFlying:            flying
elif WasInWaterFlag:        WaterTravel + LiquidTravel
elif not ImmuneToLavaDrag and containsMaterial(shrink(aabb, 0.1, 0.4, 0.1), Lava)   # region vt+0x40, 14a72d520
                            LavaTravel + LiquidTravel
elif gliding flag:          Gliding
elif ground (OnGround / ...): Ground   else Air
```

Each tick, for WaterTravel (**[V]** per system; **[I]** that they run in registration order, which is
`TravelMoveRequest` (move) -> registerPostMoveTravelVelocitySystems: `MobMovementDrag::Water/Lava` (141c59d00) ->
the gravity systems (141c5ca50) -> air drag (141c5a400)):

```
# before the move: WaterMoveSystem (moveRelative)
v += rotate_yaw(input) * travel_speed / max(1, |input|)      # only if |input|^2 >= 1e-4
                                                             # no AI -> input = 0 -> nothing
move(v)                                                      # collisions
# post-move:
f = SPRINTING ? 0.9 : (WaterMovementComponent ? drag_factor : 0.8)
f = f + (0.546 - f) * depth_strider / 3   (halved off-ground; skipped if swim_speed_multiplier > 1)
v.x *= f; v.z *= f; v.y *= 0.8                               # WaterDrag::apply 141c67400
g = -0.02                                                    # MobWaterGravity 141c5c370 -> ApplyGravity
v.y += g                                                     # ApplyGravity 141c867b0
```

- Gravity exception (141c5c370): no gravity when the mob "can't sink" (Navigation `can_sink` false, or no
  Navigation but has the gravity-gate component and the `HAS_GRAVITY` (49) flag clear) **and** the cells at
  `floor(pos.y + h/2)` and `floor(pos.y - h/2)` (h = box height, same x, z) both have a non-Air material. A
  pig (`navigation.walk`, `can_sink` default true) gets `-0.02`. **[V]** constants and branches, **[I]** which
  optional component is the gate.
- `underwater_movement` / `minecraft:movement` only feed `WaterTravelSystem` (148456490), which writes
  `MobTravel.speed` (lerp toward the base speed by depth strider; `> 1` multiplier scales it). That speed only
  multiplies the input in `WaterMoveSystem`, so with no input it has **no effect**. **[V]**
- Climb out (1424e5cb0): `v.y = 0.3` when the box moved by `(v.x, v.y + 0.6 - dy, v.z)` has no collision and no
  liquid, like Java's `isOffsetPositionInLiquid`. Gating (horizontal collision) not read. **[I]**
- No buoyancy for ordinary mobs: there is no upward term in any of the above. `BuoyancySystem` exists (for
  `minecraft:buoyant` entities) and was not read. **[I]**
- Net for a no-AI pig in still water: `vy' = vy * 0.8 - 0.02` (terminal -0.1), `vxz *= 0.8`. Same numbers as
  Java's `LivingEntity.travel` water branch (`*0.8`, `-0.02`), also in the same order (drag, then gravity, after
  the move).
- When the shrunk band `[feet+0.401, top-0.401]` leaves the water cells (e.g. a pig whose feet are above
  `cell+0.6`), WasInWater is off and the mob uses the air rules. The air gravity/drag was not read here.

Lava (mobs, LavaTravel) **[V]** constants, **[I]** gating details:

```
move(v); v *= 0.5 (all three axes; MobMovementDrag::Lava 141c69960)
v.y += -0.02 (MobLavaGravity 141c5c610), same can_sink/float exception;
     skipped for can_walk_in_lava (Navigation bit 7), which use LavaWalkAndSinkGravity (-0.08 or -0.01)
```

## 3. Falling blocks in water [V]

`FallingBlockActor` tick (145984830): `v.y -= 0.04; move(v); v *= 0.98`, then landing. No material, water flag or
head flag is read anywhere in it (no `cmp [material], 5`, no water component hash). A falling block has no
`MobTravelComponent`, so the WaterTravel drag/gravity systems never touch it (**[I]**, from the travel view).
So it falls through water exactly as through air: gravity 0.04, drag 0.98. It still gets the splash sound
(InWaterSensing runs for every actor) and concrete powder still hardens (falling-blocks.md).

## 4. Experience orbs [V]

`ExperienceOrb::normalTick` (142f9d480): the only material test is `getMaterial(floor(pos)) == Lava (6)` at
142f9d8f5 (then `v = ((r-r)*0.2, 0.2, (r-r)*0.2)` and Fizz). There is no water test inside it. **But orbs do
float**: `xp_orb.json` has `minecraft:buoyant`, handled by the generic BuoyancySystem. See 5 (corrected after a live
capture contradicted the first version of this section).

## For sapling

- Compute once per tick, before any movement, from the post-move position of the last tick:
  `water_in_box`, `lava_in_box`, `material`, `head_in_water` (formulas in 1). Use the liquid layer of a block
  (waterlogged counts). Items: unshrunk box, head point `feet + 0.375`.
- Items: the existing `postNormalTick` water branch keyed on `head_in_water` (items.md).
- Mobs without AI in water: after the move, `vxz *= 0.8` (or `drag_factor`, 0.9 sprinting), `vy = vy*0.8 - 0.02`.
  In lava: `v *= 0.5; vy -= 0.02`. Ignore `underwater_movement` while there is no input.
- Falling blocks: no water rule. Orbs: the buoyancy rule in 5, run before the orb's own tick.

## Unverified / inferred

- System execution order equals registration order (both the pre-tick water group and the post-move travel group).
- `InterpolateMovementNeededComponent` being on every server actor (it gates the request reset).
- `getLiquidBlock` (vt+0x28) and `getMaterial` slots: from the LeviLamina vtable order, not from a callee read.
- Which blocks have Water material besides `water` / `flowing_water` (bubble column assumed Bubble).
- Mob breathing offsets; the gravity-gate optional component in `MobWaterGravity`; ClimbOutOfLiquid gating;
  mob air gravity/drag.

## 5. Buoyancy (`minecraft:buoyant`): why xp orbs float

`xp_orb.json` (vanilla behaviour pack brarchive) is
`"minecraft:buoyant": {"apply_gravity": false, "liquid_blocks": ["minecraft:flowing_water", "minecraft:water"]}`,
with every other field at its default. Defaults (definition parser 141b85560, data 14a774660): `base_buoyancy` 1.0,
`apply_gravity` true, `movement_type` `"waves"`, `big_wave_probability` 0.03, `big_wave_speed` 10. **[V]**

| VA | What |
|---|---|
| 148388ab0 | `BuoyancySystem::registerSystems`: `IncreaseBuoyancyTimer`, `CheckAndAddBuoyancyFloatRequest`, ... |
| 1483880b0 | `checkAndAddFloatRequest`: request = {canFloat, needToResurface} when either is true |
| 141b8b220 | `BuoyancyComponent::canFloat` |
| 141b8b390 | `BuoyancyComponent::needToResurface` |
| 1483a9b40 | timer step |
| 1483a91e0 | the float velocity rule |
| 1483ac910 | buoyancy gravity: `vy = (vy - 0.04) * 0.98` when `apply_gravity` (off for orbs) |

Per tick, before the orb's own `normalTick` **[V]** for the code, **[I]** for that order (the capture fits only that
order):

```
c = floor(pos)                                   # pos = StateVector pos (an orb's wire position)
L = getLiquidBlock(c); U = getLiquidBlock(c + up)
in_list(B) = B is one of liquid_blocks (water, flowing_water)
can_float        = pos.y < surface(L, c) and in_list(L) and not in_list(U)   # in the top liquid cell, under its surface
need_to_resurface = in_list(L) and in_list(U)                                 # a liquid cell above: deep
# surface(L, c) = c.y + 1 - (depth < 8 ? depth/9 : 0), the same 142ed6bc0 as the head test

timer += movement_type == bobbing ? 1
       : 0.05 * (1 + 30 * |v.xz|) * (rand < big_wave_probability ? big_wave_speed : 1)

if can_float:
    b = clamp((base_buoyancy - (pos.y - floor(pos.y))) * 0.9 + 0.1, 0, 1)
    w = waves:   0.035 * (sin(timer) + 1)        # 0 .. 0.07
        bobbing: 0.5 * (sin(timer * 0.31415927) + 1.2)
        none:    0
    if b == 0: skip
elif need_to_resurface:
    b = 1; w = 0
else: skip
v.y *= 0.7
target = (b - w - 0.1) * 0.15
v.y = min(target, v.y + 0.05)       # an alternative branch max(target, 0.05*v.y) for v.y > 0 needs a
                                    # level flag and actor flag byte3 & 0x40; not taken in the capture
# buoyancy never touches v.x, v.z
```

Then the orb's own tick: `v.y -= 0.04; move; v.xz *= 0.98 (blockFriction*0.98 on ground); v.y *= 0.98`.

**The two regimes** in the capture are the two operands of the `min`, not deep vs shallow water:

- "Deep" (`v' = 0.686 v + 0.0098`): `0.7v + 0.05` is the smaller operand, so `v' = 0.98 (0.7v + 0.05 - 0.04)`.
  The 0.7 is the buoyancy damping and the 0.98 the orb's drag. **[V]** It matches the capture to 5 digits.
- "Near surface": `target` is the smaller operand, so
  `v' = 0.98 (0.15 ((1 - frac) * 0.9 - w) - 0.04)` with `frac = pos.y - floor(pos.y)`.
  `target` falls by 0.135 per block of rise and `w` changes with the timer, which gives the ~0.88 decay.
  Backing `w` out of capture ticks 15..18 gives 0.0320, 0.0292, 0.0266, 0.0240, i.e. `0.035 (sin t + 1)` with t
  advancing ~0.0775 per tick = `0.05 * (1 + 30 * 0.018)`, the orb still drifting at ~0.018 b/t sideways. **[V]** fit.
- Boundary: the switch happens the first tick `0.7v + 0.05 > target`.
- `need_to_resurface` (b = 1, target 0.135) applies whenever the cell above is also water, so a submerged orb
  rises at up to `0.98 * (0.135 - 0.04) = 0.093` b/t.
- Settle: `v = 0` needs `target = 0.04`, i.e. `(1 - frac) * 0.9 - w = 0.2667`, so
  `frac = 1 - (0.2667 + w) / 0.9`. That puts `pos.y` between `surface - 0.37` (w = 0.07) and `surface - 0.30`
  (w = 0). It is not a fixed point: the orb bobs with the wave. Period `2*pi/0.05` ≈ 126 ticks at rest. The
  captured -19.303 under a -19 surface is `w` ≈ 0.006, near a wave trough.
- Horizontal: nothing in water. Only the orb's 0.98 (air) or `blockFriction * 0.98` (ground). **[V]**
- Above the surface (`pos.y >= surface`) or out of listed liquid: no buoyancy, the orb's plain gravity.

Other `minecraft:buoyant` users (boats etc.) get the same rule with their own settings. Whether the dropped item
(`minecraft:item`) has the component was not checked; items.md's hard-coded rule covers items.

## 6. One-tick lag of the water state (armor stand capture)

The capture (armor stand on air rules for the tick starting at feet -19.59, although its shrunk band
`[-19.19, ..]` already reached water cell -20; water rules only from the tick starting at -20.11) means that
`TravelTypeSensing` in tick N reads the `WasInWater` computed from the box at the **start of tick N-1**.

What the binary says **[V]**:

- The water group (`UpdateWaterStateRequest`, `LiquidPhysics`, `InWater`/`UnderWater` sensing, splash) is added
  through 1418f9430 / 1418f97c0, which take static per-type ids (ordering constraints). The travel group
  (`TravelTypeSensing` 1418f2774, `TravelMoveRequest` 1418f2b93, post-move velocity 1418f2c2b,
  `ActorPostAiStepTick` 1418f46e2, all in 1418eeea0) is added through 1418e7270 / 1418e7380.
- So registration order does **not** give the execution order, and section 1's "water state first" is withdrawn.
  The constraint table was not decoded.

Consistent with the capture **[I]**: per tick, `TravelTypeSensing` (reads last tick's flags) -> water state from
the current (pre-move) box -> move -> post-move drag/gravity. For mobs, sapling should use the water/lava flags
computed one tick earlier: the box at the start of the previous tick. Not settled for items (their
`HeadInWater` is read in `ActorPostAiStepTick`, after the AI step; whether the water group ran earlier in the same
tick is not known). The orb capture fits buoyancy evaluated on the position at the start of the same tick (lag 0),
but buoyancy does its own cell test and does not use the water flags.

## 7. Checked on BDS (2026-10-08, `bds_command_probe water`, research/captures/{bds,sapling}-water.txt)

An item from `/setblock .. air destroy`, a `/summon`ed orb and armour stand, dropped into still water three deep.

- Armour stand: sapling's path matches BDS's to 1e-4 through the fall, the sink and the landing, with the mob's
  speed worked out **before** its move (`v.y = (v.y - 0.08) * 0.98` in the air, `v = v * 0.8; v.y -= 0.02` in
  water) and the water state read a tick late (section 6).
- Item: in water its moves follow `v' = 0.9702 v + 0.00108` on both servers, so the item rule of section 1 holds.
- Orb: in water `v' = 0.686 v + 0.0098` on both, settling about 0.31 under the surface. The paths differ only by
  `/summon`'s random starting speed.
- Moves sent: BDS's rule of items.md 5 (distance over 0.01 in all, or 0.001 once all but stopped, every changed
  axis), which `sync.rs` now follows: the orb's sends of 0.0100 to 0.0101, the item's of 0.007 at the bottom of a bob.
- Flags (research/captures/bds-water-flags.txt): an item's `AFFECTED_BY_GRAVITY` is cleared on each tick water lifts
  it and set again on each tick it bobs out, sent with that tick's move. `MOVING` shows whether the tick before moved
  it more than 0.0316 (`moving-flag.md`), which an item slowly rising in water does not.

## 8. Lava, checked on BDS (2026-10-08, `bds_command_probe lava`, research/captures/bds-lava.txt)

The water drill over lava. sapling: `body/liquid.rs`, `drops.rs`, `splash.rs` (the fizz).

- Item: lifted from the first tick its unshrunk box touches lava, by the water rule of section 1 but `v.xz *= 0.95`
  (matched to 1e-4). That tick's flags clear `AFFECTED_BY_GRAVITY` and set `ONFIRE`, with its move. It fizzes from
  the middle of its box where the tick started, heard a tick late. On its second tick in lava it burns (5 health, 4
  a tick): no move is sent, and the next tick brings the second fizz and `RemoveEntity`.
- Orb: no buoyancy in lava. With its feet in a lava cell its motion becomes `((r-r)*0.2, 0.2, (r-r)*0.2)`, then it
  moves (`dy` exactly +0.2) and drags; it fizzes as the item does and is never set on fire.
- Armour stand: `v *= 0.5; v.y -= 0.02` by the state a tick late, as in water (terminal -0.04, exact). It gets
  `ONFIRE` and a `HurtAnimation` every 10 ticks from the tick its box reached lava, and was still alive after 8 s.
  sapling has no mob health and does neither.
