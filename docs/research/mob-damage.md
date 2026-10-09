# Mob health, fire and lava

2026-10-09, `bds_command_probe burn`, research/captures/bds-burn.txt (and bds-suffocate.txt, the same drill
with its mobs summoned into a wall). No decompile: all of this is from the captures. Built in
`sapling-shard/src/creatures/harm.rs`; sapling's own run of the drill is research/captures/sapling-burn.txt.

## What viewers are told

- A hit: `EntityEvent` `HurtAnimation`, its `data` the cause (BDS's `ActorDamageCause`: 4 a block the mob is
  in, 6 touching fire or lava, 7 burning on, 8 lava), then `UpdateAttributes` with `minecraft:health` alone
  (`min` 0, `max`, `default_max` and `default` its full health). No sound is sent.
- The hit that kills sends `DeathAnimation` in place of `HurtAnimation`; health 0 follows.
- Fire's health is told in the tick of the hit, lava's a tick after its hit: health is told between the two.
- On fire: `SetEntityData` with the mob's flags and `ONFIRE` (bit 0), cleared when the fire ends.
- `/kill` tells health 0 a tick after the death.

## Rules

- A mob whose box reaches into a cell of lava or fire (the whole cell, whatever the lava's height; a box
  resting on top of the cell does not) is set on fire and hit for 1 (cause 6) each tick.
- A mob in lava as its movement counts it (`body/liquid.rs`, its box less 0.401 at top and bottom) is hit
  for 4 (cause 8) each tick, after fire's hit.
- A hit starts 10 ticks in which only a harder hit lands, and for the difference; that harder hit starts the
  10 ticks again. So a pig put over lava: 9 on its 1st tick (fire), 6 on its 3rd (lava's 4 less 1), then
  5 and 2 on the 13th, 1 and dead on the 23rd.
- Out of fire, it burns 160 ticks more, hit for 1 (cause 7) every 20: 7 hits, then the flag clears.
- Water puts the fire out at once, with a `ParticleFizzEffect` `LevelEvent` and a `fizz` sound (sapling
  sends neither).
- A mob that dies on fire leaves its meat cooked (the loot tables' `furnace_smelt` under `on_fire`).
- The kinds fire, falls and water leave alone are those whose pack components say so
  (`sapling-data/data/mob-traits.tsv`). Not drilled: the drill world refuses `/summon zombie_pigman`.

## Falls, blocks and water (2026-10-09, `bds_command_probe harm`, research/captures/bds-harm.txt)

- A fall costs each block begun past 3: cows dropped 3, 3.5, 4, 4.5, 5 and 8 blocks lost 0, 1, 1, 2, 2 and 5;
  14 and 30 killed. The hit comes the tick after the move that lands.
- It is told as `HurtAnimation` with cause 5 and `fire_at_position` the place landed on, then an
  `EntityEvent` `DustParticles`, then health; the fall that kills sends `DeathAnimation`, the dust, health 0.
- A tick later two sounds, both from where the mob was before the move that landed it, both naming the
  mob: `fall.small` (`fall.big` when it cost more than 4) with -1, and `fall` with the hash of the block
  landed on. A 3 block fall sends nothing.
- A mob whose head is in a block (a cow under a roof one block up; not one whose drawn head, forward of its
  feet, is in a wall) is hit for 2 (cause 4) from about the 6th tick of it and every 10 after, told with its
  health in the same tick.
- With its head in a block or in water, `Air` in `SetEntityData` counts down from 300 by 1 a tick.
- At `Air` 0 under water it is hit for 2 (cause 9) every 10 ticks, the health told a tick after each.
- Under water a `LevelEvent` `ParticleBubble` (data 8) comes from its drawn head every tick.

## Where sapling differs

- A chicken falls slowly on BDS and reaches the lava on its 8th tick; sapling's falls as any mob, on its 3rd.
- BDS's loot appears the tick after the death; sapling's with it.
- A mob left inside a block (lava under it made stone) is lifted onto it on BDS and stays in it on sapling.
- A cow shut in water swims up on BDS; sapling's sinks. Both drown at the same tick.
- Any full block smothers on sapling, and a fish out of water breathes.
- BDS also flips `MOVING` (bit 34) and another flag (bit 35) on mobs; sapling sends their recorded flags.

## Not drilled

How breath comes back out of water (sapling gives all of it at once), a fall into water or onto slime, hay
or a bed, which blocks do not smother (glass, leaves), fish out of water, soul fire, magma, campfires, a fire block's burn time against lava's, rain, fire resistance, armour, a baby,
the armour stand's own burning (bds-lava.txt: hurt animations with data -1), players.
