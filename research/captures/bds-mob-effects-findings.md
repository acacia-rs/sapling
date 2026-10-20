# bds-mob-effects findings

Capture: `research/captures/bds-mob-effects.txt` (drill `mob_effects`). Times are the capture's first
column (ms), ~50 ms a tick. Mob packets carry `tick: 0`; ticks are counted from MoveEntityDelta (one a
tick while a mob moves) and the SyncWorldClocks stamps (every 256 ticks). The actor and watcher
streams match packet for packet for the mobs (same counts per type, Text/CommandOutput aside).
Two capture stalls batch packets: ~490 ms around 19.4-22.4 s (trial 1; clock 18378 -> 31671 = 13293 ms
for 256 ticks) and 58934 -> 60651 (34 cow moves in one batch). Intervals below take those into account.
Cause codes are ActorDamageCause: -1 none (/damage), 14 magic, 15 wither.

## Packets for a mob's effects

- **MobEffect goes out only for speed and slowness** (the two movement-attribute effects): 3 Adds in the
  whole capture, each in both streams, on the command tick:
  `{ event_id: Add, effect_id: 2, amplifier: 255, particles: true, duration: 1200, ambient: false }`
  (slowness 60 s), `effect_id 1, amp 1, duration 600` (speed 30 s), `effect_id 2, amp 255, 600`.
  Regeneration, poison, wither, instant health/damage, resistance, fire resistance and water
  breathing send **no MobEffect at all**. No Update or Remove appears: every speed/slowness mob was
  killed before expiry, and death sends no Remove either.
- **SetEntityData VisibleMobEffects** is the one packet every effect gets, 1 tick after the command,
  and again with the next value when an effect ends. Each effect is a 7-bit slot `(id << 1) | ambient`;
  slots are packed in ascending effect id with the lowest id in the highest slot
  (`v = v << 7 | slot`):

  | effect (id) | value | combined |
  |---|---|---|
  | speed (1) | 2 | slowness + instant damage = **526** = 4<<7 \| 14 |
  | slowness (2) | 4 | slowness + instant health = **524** = 4<<7 \| 12 |
  | instant health (6) / damage (7) | 12 / 14 | slowness + fire res (fire res first) = **536** = 4<<7 \| 24 |
  | regeneration (10) / resistance (11) | 20 / 22 | |
  | fire resistance (12) / water breathing (13) | 24 / 26 | |
  | poison (19) / wither (20) | 38 / 40 | |

- Instant effects sit in VisibleMobEffects for one tick (14 at 92084, 0 at 92133; 12 and 0 in the same
  97043 batch). After the kill, VisibleMobEffects 0 is sent ~10 ticks later (178088 vs death 177533).
- **Movement attribute** (UpdateAttributes on the mob, 1 tick after the command, movement only):
  speed II -> `movement 0.35`, modifier `"MovementSpeed", amount 0.4, operation 2, operand 2`
  (0.2 x level on base 0.25). Slowness 255 -> `movement 0.0`, `"MovementSlowdown", amount -38.4, op 2`
  = **-0.15 x 256: amplifier 255 counts as level 256** (no signed-byte wrap), and the value clamps at 0.
  Modifier UUIDs are fixed (`91aeaa56-...` speed, `7107de5e-...` slowness; same in both slowness trials).
- Extra flags: **fire_resistance sets Flags bit 50 (FIRE_IMMUNE)** with VisibleMobEffects in one
  SetEntityData. **water_breathing sets bit 35 (BREATHING) and Air 300** in the same packet.
- Particles: no LevelEvent/particle packets. `particles: true` in MobEffect and VisibleMobEffects are
  all the client gets; swirls are client-drawn.

## Health changes

Every tick effect fires **on the application tick** (same packet batch as VisibleMobEffects), then
every interval: fires while `remaining % interval == 0`, not at 0.

| trial | effect | events (ms) | interval | change | count |
|---|---|---|---|---|---|
| 1 cow 5/10 | regen 10 s amp 0 | 16934, 19433, 22418*, 24924 | 50 ticks | +1 each, no hurt event | 4 (200/50), ends at 9 |
| 2 cow 10/10 | poison 15 s amp 0 | 33973, 35223 ... 43967 | 1247-1256 ms = 25 ticks | -1, HurtAnimation 14 | 9, stops at **1.0** |
| 3 cow 10/10 | wither 10 s amp 0 | 55486, 57484, 59484**, 61484, 63482 | 40 ticks | -1, HurtAnimation 15 | 5 (200/40), ends at 5 |

\* 22418 is the trial-1 stall (expected 21934). \*\* sent in the 60652 batch, 10 cow moves after 58934.

- Poison floor: health 1.0 from 43967 to the end of the effect (48969), **5 s with no more damage**:
  poison never goes below 1. Wither from 10 health only reached 5 in 10 s, so **whether wither kills is
  not tested** (needs > 400 ticks or amp 1+).
- Effect ends (VisibleMobEffects 0): regen +10483 ms (stall), poison +14996, wither +10002.
- Every magic/wither hit makes the cow panic: speeds settle at **0.2108** = 2.1586 x (0.25 x 1.25)^2.
- No hurt/heal sounds for any effect damage.

## Instant effects

- Cow: instant_damage **10 -> 4 (-6)**, HurtAnimation 14. instant_health **4 -> 8 (+4)**.
- Zombie (20 max): `/damage 8` -> 12.128 (**7.872 = 8 x (1 - 0.4/25)**: armor 2 through the Java
  armor formula, even for `/damage`, cause -1). Then instant_damage **12.128 -> 18.128 (+6)** and
  instant_health **18.128 -> 14.128 (-4)**: undead swap the sign but **keep the effect's own
  magnitude** (Java heals 4 / hurts 6 instead). The -4 skips armor and sends **no HurtAnimation**.
- Zombie poison 5 s and regeneration 5 s: CommandOutput success, but **no packet at all**
  (VisibleMobEffects stays 4 = slowness only, no health change in 7 s): BDS never adds them to undead.

## Resistance, fire, water

- Resistance II (amp 1) then `damage 4`: **8 -> 5.6 (2.4 taken, a 40% cut)** = 20% a level.
- Fire trial is **inconclusive**: the zombie stood in the fire cell (3.5, -19, 0.5) for 1 s before
  fire_resistance and 10 s after, with no ONFIRE bit (Flags bit 0 never set), no hurt event, no fire
  sound. Cleanup `/setblock ~3 ~ ~ air` reported `noChange`: the fire block was gone. Redo with
  netherrack under the fire, or the fire placed after the summon.
- Water: the cow spawned in the pit (feet -22) and Air dropped **1 a tick from 300** (300 -> 280 by
  148017, 20 ticks). water_breathing at 148033: **Air straight back to 300** and bit 35 set, Air never
  sent again. But the cow **floated up** (dy +0.02 -> +0.099 a tick, feet -19.9 by 148482) and bobbed at
  the surface (feet -20.1..-18.5) for the rest, head out, so **drowning was never tested**: no hurt in
  29.5 s means nothing. Redo with a 1-block ceiling over the water.

## Speed and slowness motion

- Plain cow stroll terminal: **0.086343** a tick = 2.1586 x (0.25 x 0.8)^2.
- Speed II cow stroll: **0.169232** (32 ticks at 0.169) = 2.1586 x (0.25 x 0.8 x 1.4)^2 = 0.169234.
  The modifier multiplies the movement attribute; the goal's 0.8 then applies on top.
- Slowness 255 zombie: no move at all (trial 4). Trial 7 zombie was walking when it landed: steps
  0.0413 -> 0.0226 -> 0.0123 -> 0.0104 (x ~0.546 friction, no new push), then still.

## Surprises

1. No MobEffect packets except speed/slowness; VisibleMobEffects carries everything else.
2. Undead instant effects keep their magnitude (+6 from harm, -4 from heal), and the heal-hurt sends no
   HurtAnimation.
3. Poison/regeneration on undead are refused silently (command still reports success).
4. Amplifier 255 is level 256 (slowness -38.4), not -1.
5. Tick effects fire on the application tick; regen 10 s gives 4 heals, wither 10 s gives 5 hits.
6. `/damage` goes through armor (zombie armor 2).
7. fire_resistance sets FIRE_IMMUNE (bit 50); water_breathing resets Air to max at once.

## Second run (`mob-effects2`, `bds-mob-effects2.txt`)

The two this run could not judge, with a mob left without the effect beside each:

- Fire: a zombie on netherrack's fire was hurt by fire (HurtAnimation 6) every 10 ticks from the
  tick it came, and died 10.5 s on. A second one, hurt twice in the second before
  `fire_resistance`, was hurt no more for the 9.5 s after it. Fire resistance stops fire damage.
- Water: a cow kept under water by glass counted `Air` down from 300 a tick at a time, was hurt by
  drowning (HurtAnimation 9) every 10 ticks once it reached 0, and died 17 s on. A second one given
  `water_breathing` took no harm in its 30 s. Water breathing stops drowning.
