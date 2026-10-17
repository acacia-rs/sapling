# Spawn LevelSoundEvent position (BDS, bdsre)

Read 2026-10-07 from `acacia/.testserver/bds-blocks/bedrock_server.exe` (PE stamp 6ab54bee) with a scratchpad copy of
bdsre plus capstone window scans. Layouts from LeviLamina `OffsetsComponent.h`, `GetAttachPositionUtility.h`,
`ActorHeadRotationComponent.h`, `AABBShapeComponent.h`. No labels added.

| VA | Suggested label | What |
|---|---|---|
| 1418dc160 | spawn-sound hook `(?, Actor&, ActorInitializationMethod)` | Spawned(2): `playSound(getSpawnSound(), getAttachPos(Head))`; Born(3): `Born` (223) at Head |
| 1429e43c0 | `Mob::getSpawnSound` | `Spawn` (53), or 12 (`AmbientInWater`) if component 0x89718a63 present and no `IgnoreHeadInWaterForSpawnSoundComponent` (0x8e298085) |
| 140d2b6d0 | `Actor::getAttachPos` (labelled) | Body/Weapon: AABB centre; else `_getBaseAttachPoint` |
| 142f50120 | `GetAttachPositionUtility::_getBaseAttachPoint` | switch on `ActorLocation` |
| 140d293d0 | `Actor::playSound(event, pos, data=-1, ...)` | fills entity_type, is_baby, unique id |
| 141cd3340 | set relative-offset config `(ctx, head, eyes, breathing, mouth)` | each arg `{wRatio, hRatio, bool set}` |
| 142c0cb80 | `UpdateBoundingBox::onAABBRelativeSizeChanged` | offsets = ratios x `mBBDim` |
| 1429df250 | `Mob::Mob` (inferred) | sets the Mob defaults below |
| 142a3f3a0 | `Animal`-family ctor (ICF-folded, vtable 14a7c3140) | sets the Animal defaults below |

## Formula (verified)

```
p      = StateVector.posPrev        # getAttachPos alpha = 0 -> lerp gives posPrev; == pos on the spawn tick
theta  = ActorHeadRotation.mYHeadRot (0 if absent), degrees
(ox, oy, oz) = OffsetsComponent.mHeadOffset = (0, hRatio * h, wRatio * w)
sound.x = p.x + ox*cos(-theta) + oz*sin(-theta)   = p.x - oz*sin(theta)
sound.y = p.y + oy
sound.z = p.z + oz*cos(-theta) - ox*sin(-theta)   = p.z + oz*cos(theta)
```

- `w, h` = `AABBShapeComponent.mBBDim` = collision_box x scale (baby pig 0.9 x 0.5 = 0.45).
- VanillaOffset (subtracted) and riding offset (added to y) are 0 for an unridden fresh mob; not modelled.
- The hook fires only when `actor+0x210 & 2` (set by the Mob/Animal ctors; armor stand passes too).

## Per-kind ratios (hard-coded in C++ ctors, not behaviour-pack data)

Rows are `(wRatio, hRatio)`; `mouth` etc. listed for completeness, only `head` matters here.

| Who | head | eyes | breathing | mouth | Confidence |
|---|---|---|---|---|---|
| Mob::Mob (armor stand, any Mob not overriding) | (0, 0.85) | (0, 0.9) | (0, 0.9) | (0.9, 0.9) | verified, matches armor stand |
| Animal-family ctor 142a3f3a0 / inlined copies | (0.85, 0.85) | (0.85, 0.9) | (0, 0.85) | (0.9, 0.85) | verified, matches chicken + baby pig |

Checks: chicken 0.6 x 0.8 -> (0, 0.68, 0.51); baby pig 0.45 x 0.45 -> (0, 0.3825, 0.3825); armor stand
0.5 x 1.975 -> (0, 1.67875, 0). All three match the capture.

Animal-default ctors found through factory registrations (ActorType low byte, names inferred from the
Bedrock numeric ids): 10 chicken, 11 cow, 16 mooshroom, 18 rabbit, 22 ocelot, 23 horse, 29 llama, 74 turtle,
75 cat, plus 0x7a, 0x80, 0x82, 0x8a, 0x8b (names not checked). Pig was not mapped to its lambda but the capture
proves it uses the Animal head. Ctors that override after the Animal default (head not checked; read the
call to 141cd3340 before using them): panda (113, 1446f4240), 0x8e (142a3fb50), 0x93 (142a46630). Other ctors with
their own tables: 0xb8c (141ec5ef0, head (0, 0.7)), 0xb92 (141ec6260, all (0, 0.85)), 0x236c (1446f6d50, one (0, 0.5)),
0x2395 (142a4c210, negative w ratios). All set calls: `callers 141cd3340`.

## For sapling

`head_offset(kind) = (0, hR*h, wR*w)` with `(wR, hR) = (0.85, 0.85)` for Animal subclasses and `(0, 0.85)` for
other Mobs; sound at `pos + (-oz*sin(yaw), oy, oz*cos(yaw))` with yaw = head yaw (summon yaw for a fresh mob). Keep
the ratio per kind in a C++-derived table, since no pack file carries it.
