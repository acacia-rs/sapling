# Warden (`bds-warden.txt`, drill `warden` in crates/sapling/examples/shared/warden_drill.rs)

BDS 1.21 (`bds-molang`), easy, the actor in survival with ten more hearts, healed every second;
the watcher in creative 14 off. Times are the capture's milliseconds; a tick is 50.

## Sniffing and looking

- Summoned 10 off the still actor: `spawn` at 0.85 of its height, and the same tick
  `FlagsExtended` with bit 41 (sniffing) and `HeartbeatIntervalTicks` 40. The sniff ends 84
  ticks on (pack: 4.16 s); it stood still meanwhile.
- The actor was 10 off, past the pack's `suspicion_radius_horizontal` 6: no anger; it walked to
  the actor at 0.0952 a tick (0.21 = movement 0.3 × `investigate_suspicious_location` 0.7, by the
  legs rule) and stopped 0.9 off the actor's feet, pressing on it.
- It sniffed no more before it roared (the pack's cooldown is 5–10 s; investigating outranks it).

## Anger (`minecraft:anger_level`, `minecraft:heartbeat`)

- Touching the actor angered it once a second (Java's touch cooldown of 20 ticks): `listening`
  and heartbeat 27 (anger 35), `listening_angry`, `AmbientSoundEventName` `agitated` and heartbeat
  14 (69), then `listening_angry`, `angry` and heartbeat 10 (103), all at 0.85 of its height.
  Heartbeat = round(20 × (2 − clamp(anger / 80 × 1.5, 0, 1.5))); anger falls 1 a second.
- At 80 it roars at once: bit 20 for 85 ticks (pack 4.2 s), standing; then `TargetEid` the
  actor and `mob.warning` a tick later. No roar sound is sent (the client plays it).
- Hunting, it hit every 20 ticks for 16 on easy (30 on normal), as in bds-wander-findings.md.
- `VibrationDetected` (EntityEvent) came when a vibration (the actor's hurt) reached it, and
  `listening_angry` a tick later; two hits 20 ticks apart gave one: it is deaf 40 ticks after one.
- Hit by a player, it takes the hitter at once (bds-beasts.txt), as Java's anger of 80 + 20.

## Sonic boom (the actor caged in glass on a pillar 5 up, in a diamond chestplate)

- Bit 43 on, `sonic_charge` a tick later, at 35 ticks the actor is hurt and thrown and
  `sonic_boom` is heard (a tick later), bit 43 off at 61 ticks; the next starts 41 ticks after
  (a cycle of 102 ticks). It stood while booming.
- 6 off 40 health on easy (10 → `min(h, h/2 + 1)`) through the chestplate: armour stops none.
- Two `SetEntityMotion` the same tick: a plain hit's throw (0.4 across, away from it; 0.4 up),
  then three times that across and 0.4163–0.4179 up (one boom 0.359; not explained).
- It boomed from 15.2 off across (pack: 15 across, 20 up), at prey it could not walk to; while it
  could reach the actor it never boomed.

## Darkness

- `MobEffect` darkness (30) for 260 ticks, particles off, to the survival actor within 20, then
  `Update` every 121 ticks; it ran out 13 s after the last. Not given by sapling: sapling-api has
  no darkness effect. A 24 s gap with no update while the actor stood 16 off is not explained.

## Emerging and digging

- `summon warden ~ ~ ~ 0 0 minecraft:spawn_emerging`: `FlagsExtended` bit 40 a tick after
  `AddEntity` (with heartbeat 40), off 141 ticks on (pack 7 s); it stood meanwhile.
- No suspects about (both players in creative): it strolled, never sniffed, and 60 s after it
  came out set bit 42 (digging; pack `idle_time` 60) and was removed 112 ticks on (pack 5.5 s):
  `RemoveEntity`, no death, no loot.
