# bds-provoke2 findings

Capture: `research/captures/bds-provoke2.txt` (drill `provoke2`). Times are the capture's first
column, which is milliseconds (~50 per tick); "tick" is the actor's tick from the `tick` field of
its SetEntityMotion. Actor unique id -554050781183, runtime 1. Easy difficulty. Knockback on every
melee hit was horizontal 0.400, y 0.4.

Run problems: the panda and bee `!hit`s failed (4.55 and 4.85 blocks away), so neither was
provoked. The actor fell into a drop during the cave spider (died 75763, fall) and was killed by the
piglin before its hit (105245), so the llama, piglin and bee ran at the respawn point (y -60, not
the stone floor).

Flag names: the generated `MetadataFlags2` table in acacia-proto is off by one (its bit 0 is
SNEEZING, but flag 64 LAYING_DOWN belongs there). With that corrected, the panda's extended bit 3
is ROLLING and the llama's bit 24 is FACING_TARGET_TO_RANGE_ATTACK, which fits what they did.

## polar_bear (rt 9)

- No target before the hit. Hurt 16313, `TargetEid` = actor 16350 (same tick as its health
  drop), `mob.warning` 16394 (one tick later), ANGRY flag (bit 25) from 16394.
- Hit back 3 times: 18793, 20793, 22793 (ticks 423, 462, 501: 39 apart, ~2 s). 4 each
  (20 -> 16 -> 12 -> 8). First 2.48 s after the hit. ArmSwing EntityEvent + `attack` sound each time.
- Chased at 0.1349 a tick (= 0.98 x 0.25^2 / 0.454, so speed multiplier 1.0). It slowed to a stop
  (0.074, 0.040, 0.022 ...) in the last ~0.4 s before each swing and sat still until the swing.
- REARING (bit 39) toggles on about 0.5 s before each swing (17744, 18295, 19643, 20294, ...).

## panda (rt 13), not hit

- AddEntity metadata: Variant 0, MarkVariant 0. The full SetEntityData in the same tick (28295)
  carries Variant 3 (playful), MarkVariant 0, so the gene arrives only after AddEntity.
- Rolled: extended flag ROLLING 31493-33093 and 33143-34744. It moved a constant 0.2000 a tick,
  then 0.2596, with a jump (vy 0.27) every ~8 ticks, which carried it 4.55 from the actor, so the
  hit failed. No target, attack_once untested.

## spider (rt 14)

- No target before the hit (noon, open sky). Hurt 52554, target 52591, `mob.warning` 52647, ANGRY
  from 52647.
- Hit back 6 times: 54049, 55050, 56043, 57042, 58050, 59243 (ticks 1114, 1133, 1153, 1172, 1192,
  1216: 19-20 apart, last one 24). 2 each. First 1.5 s after the hit.
- Chased at 0.1937-0.1941 a tick (= movement 0.3, multiplier 1.0 by the same formula), dropping
  between swings as the knockback opened the gap. It jumped (vy 0.333) a few times while closing.

## cave_spider (rt 20)

- No target before. Hurt 70664, target 70693, `mob.warning` 70749, ANGRY from 70749.
- Hit back twice: 72642, 73643 (ticks 1478, 1498: 20 apart). 2 each. First 1.98 s after the hit.
- Each hit sent MobEffect Add poison (id 19, amplifier 0, duration 0) and a Remove one tick later:
  no poison on easy.
- Peak steps 0.19 a tick. The knockback took the actor west into a drop. The spider dropped its
  target at 75143, fell and died at 76644 (`fall.big`); the actor died of the fall at 75763.

## llama (rt 23, Variant 2)

- No target before. Hurt 88872, target 88893 (same packet sets ANGRY and extended bit 24,
  FACING_TARGET_TO_RANGE_ATTACK), `mob.warning` 88943.
- It did not walk at the actor: after its knockback it stood still (steps ~0.01).
- Charge: `warn` sound, CHARGED flag (bit 27), ChargeAmount 1 at 89900 (1.03 s after the hit);
  ChargeAmount 0 at 90844.
- One spit: AddEntity llama_spit at 90943 (2.07 s after the hit), OwnerEid = llama, at (65.775,
  -58.417, 33.528), llama at (66.379, -60.0, 32.725), so 1.58 above its feet and 1.0 off toward the
  actor. Velocity (-0.890, -0.167, 1.207), |v| 1.509; the next SetEntityMotion has vy -0.227
  (gravity 0.06). Steps 1.4995 and 1.4190 a tick. `shoot` sound from the llama.
- Spit hit: actor HurtAnimation data 3 (melee hits are data 2) at 91343, health 20 -> 19 (1).
  No SetEntityMotion for the actor, so no knockback. Spit removed 91393. It hit 0.4 s after
  spawning, slow for 1.5 a tick over 1-2 blocks; the spit's moves are logged for only 2 ticks.
- Target dropped at 92843 (3.95 s after the hit), bit 24 cleared at 92917. No second spit.
- When `/kill` killed it (95943), it took the actor as target again with `mob.warning`.

## piglin (rt 27, Variant 1)

- Hostile with no provocation: target 101301, 0.36 s (7 ticks) after spawn, `mob.warning` 101347,
  `angry` sound 101396. Holds item network_id 325, the same id the zombified piglin held in
  bds-provoke (golden sword).
- Flag bit 40 (VIBRATING in the generated table) is set from 100993, one tick after spawn. This is
  most likely the shaking that comes before it zombifies in the overworld.
- Hits: 102243, 103246, 104250, 105244 (ticks 2017, 2036, 2056, ~2075: 19-20 apart). 5.5 each
  (20 -> 14.5 -> 9 -> 3.5 -> dead, `death.attack.mob`). First 0.94 s after taking the target. Two
  ArmSwing EntityEvents per hit.
- Steps peaked at 0.21-0.24 a tick between hits, with no steady stretch.
- The scripted `!hit` (107222) landed while the actor was dead. After the respawn the piglin was
  out of range (removed 107545). At the same time a zoglin from another id range (-386547056632)
  was sent at (-39.3, -60, -65.7) already dead (health 0, DeathAnimation).

## bee (rt 30), not hit

- No target. It flew at 0.153-0.162 a tick, with a 0.43 burst at 124491, and was 4.85 off when the
  hit was tried. Sting and death untested.

## Not settled

Panda (attack_once, gene behaviour when hit) and bee (sting, death after sting): rerun with the
actor going to the mob in the same tick as the hit, or with a reach check that waits.
