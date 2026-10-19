# bds-provoke3 findings

Capture: `research/captures/bds-provoke3.txt` (drill `provoke3`). Times are the capture's
millisecond column; "+Nt" is ticks after the `## hit N: Ok` line (ms / 50). Actor unique id
-558345748479, runtime 1, feet at y -19, z 20.3-20.5, x = the mob's spawn x. Easy difficulty.
Flags: bit 25 ANGRY, 34 MOVING, 35 BREATHING, 6 TEMPTED; extended (`FlagsExtended`) bit 3 is
ROLLING after the off-by-one correction (see bds-provoke2-findings.md). Extended bit 25 is set on
every mob from spawn and never changes.

Common to all four hit mobs:
- HurtAnimation (data 2) on the mob +0.3..1.0t; the mob's health drop and `TargetEid` = actor
  arrive together one tick later (+0.9..1.6t); `mob.warning` + ANGRY one tick after that.
- Mob knockback from the fist: first step 0.400 horizontal, +0.400 up.
- Each mob hit the actor exactly once: ArmSwing on the mob, HurtAnimation data 2 on the actor,
  actor SetEntityMotion h 0.4000, y 0.4000, and `TargetEid` 0 all in the same packet tick; actor
  health 20 -> 18 (2) one tick later at most; `attack` sound and ANGRY cleared the next tick. No
  second attack in the remaining 5-6 s.
- `/kill` gave every hit mob the actor as target again (`TargetEid` + `mob.warning`); the unhit
  bees did not react.
- No MobEffect on the actor other than the drill's instant_health (6) and the clear.

## panda 1 (rt 3): playful

- AddEntity Variant 0, MarkVariant 0; the same-tick SetEntityData carries Variant 3 (playful),
  MarkVariant 0.
- Target +1.3t (15381), `mob.warning`/ANGRY +2.3t. Let go at +74.3t (19031), the tick of its hit.
- Hit at +74.3t (3.71 s), actor tick 427: 20 -> 18, knockback (-0.0919, 0.4000, -0.3893).
- After its own knockback (lands +11.5t) it stood ~35 ticks, accelerated from +48t and walked at a
  steady 0.0485-0.0486 a tick (MOVING on) up to the swing, from ~1.78 blocks. After the swing it
  slowed (0.0265, 0.0145, 0.0122) and stopped.
- Rolled at +167t (23682): extended ROLLING set, SetEntityMotion vy 0.27, steps 0.2000 a tick for
  7 ticks, then 0.0257 on the ground; still rolling at the /kill (+180t). No sneeze.

## panda 2 (rt 6): normal

- Variant 0, MarkVariant 0 in AddEntity and the SetEntityData; no later change.
- Target +1.6t (33831), ANGRY +2.6t. Let go +75.6t (37531) with its hit.
- Hit at +75.6t, actor tick 790: 20 -> 18, knockback (0.0323, 0.4000, -0.3987). Only one hit:
  attack_once holds for the normal gene.
- Moved first at +15t (ramp to 0.0745 at +26t, then stopped at +30t), idle to +46t, then a steady
  0.0485-0.0486 a tick from +58t to the swing at ~1.75 blocks; afterwards 0.0265, 0.0145, stop.
  Same gait as panda 1. No other flags or sounds.

## panda 3 (rt 9): lazy

- AddEntity Variant 0; SetEntityData Variant 1 (lazy), MarkVariant 0. TEMPTED (bit 6) set from
  spawn, cleared by the hurt packet.
- Target +0.9t (52235), ANGRY +1.8t. Let go +161.9t (60285) with its hit.
- Hit at +161.9t (8.1 s), actor tick 1236: 20 -> 18, knockback (-0.1531, 0.4000, -0.3695).
  One hit.
- Same 0-30t pattern as panda 2 (ramp to 0.064, stop), then from +56.9t a constant 0.0105-0.0106
  a tick (0.218 of the other pandas' 0.0486) for 105 ticks, MOVING not set, to the swing at ~1.89
  blocks; afterwards 0.0058, 0.0032. The swing came just before the /kill (+180t).

## bee 1 (rt 12), hit

- Variant 0, MarkVariant 0 at spawn; spawn flags +BREATHING, -HAS_GRAVITY (bit 49). Health 10.
- Hit: health 10 -> 9 at +1.1t with `TargetEid` = actor; ANGRY + `mob.warning` +2.1t.
- Sting at +25.2t (71787, 1.26 s), actor tick 1461: ArmSwing, actor 20 -> 18, knockback
  (-0.0755, 0.4000, -0.3928), `attack` sound +26t. The same SetEntityData sets MarkVariant 1
  (stinger gone), `TargetEid` 0, clears ANGRY and carries AmbientSoundInterval.
- No poison: no MobEffect 19 at all (easy).
- It did not die: health stayed 9, no DeathAnimation/RemoveEntity until the /kill at +182.9t
  (7.9 s after the sting).
- Before the sting: flight steps rose 0.08 -> 0.174 a tick, then a slow 0.045 drift and a dive
  (vy -0.13) into the actor. After it: no chase; wander loops of a steady 0.0404 horizontal
  with 0.0424 vertical per tick between bursts of 0.13-0.17.

## bees 2 and 3 (rt 15, rt 16), not hit

- `!hit` failed at 3.21 and 3.11 blocks.
- Variant 0, MarkVariant 0, no target, no ANGRY, no sounds besides spawn. Only MOVING toggles.
  At the /kill neither took a target (unlike every hit mob).

## Not settled

- Panda attack damage was 2 on easy for all genes; aggressive, worried, weak and brown genes
  untested. Lazy panda's 0.0106 step and its pre-hit TEMPTED flag are unexplained.
- Bee death after stinging needs a longer window than 8 s.
