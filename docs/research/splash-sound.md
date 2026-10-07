# Water splash LevelSoundEvent (BDS, bdsre)

Read 2026-10-07 from `acacia/.testserver/bds-blocks/bedrock_server.exe` (PE stamp 6ab54bee) with a scratchpad copy of
bdsre plus capstone window scans. Layouts from LeviLamina `InWaterSensingSystem.h`,
`UpdateWaterStateRequestComponent.h`, `WaterSplashEffectRequestComponent.h`. No labels added.

| VA | Suggested label | What |
|---|---|---|
| 140d52050 | `Actor::doWaterSplashEffect` (Actor vtable slot 130, +0x410) | the sound; folded into most actor vtables |
| 1484043d0 | `InWaterSensingSystem::fn0` (labelled) | `mMaterialType == 5` (water) and no `WasInWaterFlag` -> add flag, request = 1 |
| 14661bdd0 | `LiquidSplashRequestSystem::fn0` (labelled) | head enters water -> request = 2, head leaves -> 3; spectator drops the request |
| 14662f670 | `LiquidSplashSystem::tick` (labelled) | per request calls 146630690 |
| 146630690 | splash request dispatch | `if !actor[0x269]`: vcall table 14a8be648 `{1: +0x410, 2: +0x400, 3: +0x408}` |
| 140d51ea0 | `Actor::updateWaterState` (inferred) | emplaces `UpdateWaterStateRequestComponent`, runs its system group synchronously |
| 14a72ca58 | factor table | `[0.9, 0.2]`, indexed by `controller == self` |

Slots +0x400 / +0x408 (`doEnterWaterSplashEffect` / `doExitWaterSplashEffect`) are `ret` (140075fb0) for Actor;
Player overrides them. Only request 1 makes the `splash` sound for non-players.

## Formula (verified against the falling-block capture to the integer)

```
controller = first controlling passenger, else self      # passenger walk at 140d52174..2b9, details inferred
f      = controller == self ? 0.2 : 0.9
v      = self.StateVector.mPosDelta                       # actor+0x218, +0x18; the vehicle's own, not the rider's
volume = min(1, sqrt(0.2*v.x^2 + v.y^2 + 0.2*v.z^2) * f)  # all f32
extra_data = (i32) trunc(volume * 16777215.0)             # cvttss2si
position   = AABB centre of self: (min + max) * 0.5       # actor+0x220 AABBShapeComponent
sound      = playSound-style broadcast, event 26 (Splash), entity_type = self's identifier, is_baby_mob = false (constant 0)
```

Falling block check: last move delta `vy = -0.4927161` (f32, before the 0.98 drag) gives exactly 1653280; the
dragged value (-0.4828618) gives 1620215, the f32 position difference gives 1653286. So `mPosDelta` is read inside
the tick's `move()`, after the position update and before the falling block's `v *= 0.98` (145984a58). The position
-60.35691 is the post-move AABB centre (= the wire y, a falling block's wire y being its box centre).

Pig (no movement in capture): 213003, 417055, 361213 -> weighted speed `sqrt(0.2vx^2+vy^2+0.2vz^2)` of 0.0635,
0.1243, 0.1076; plausible for a swimming pig, not checked.

## When it fires

```
in_water   = UpdateWaterStateRequest.mMaterialType == Water(5)      # set by the water-state update
             and not (passenger of an entity with BoatFlag + VehicleComponent)
if in_water:
    if no FallingBlockFlag: fallDistance = 0
    if not WasInWaterFlag: add WasInWaterFlag; WaterSplashEffectRequest = 1
else: remove WasInWaterFlag
```

- So one splash per entry: the first tick the water-state check says "in water". For the falling block (water
  cell y -61, surface -60.111) it fired on the move that took the box bottom to -60.847, not on the previous one
  that left it at -60.354, already below the surface. That fits a Java-1.12-style check on the box shrunk 0.4 from the bottom (`grow(0, -0.4, 0)`), not
  read in the binary. Confidence: medium.
- If the head enters water in the same tick, `LiquidSplashRequestSystem` may overwrite the request with 2, and an
  Actor's slot +0x400 is empty, so no sound. Which of the two systems runs last was not read. Confidence: low.
- `actor+0x269` set (inferred: removed/dead) skips it.

## For sapling

On the first tick a non-player entity's water check turns true (and it is not riding a boat), during its move
and before drag: send `LevelSoundEvent { sound: splash, pos: aabb_centre, extra_data:
(min(1, sqrt(0.2*vx^2 + vy^2 + 0.2*vz^2) * 0.2) * 0xFF_FFFF) as i32, entity_type, is_baby_mob: false, unique id }`
with `v` the motion just used for the move. Use 0.9 instead of 0.2 when the entity has a controlling rider.
