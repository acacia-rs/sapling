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
- Water puts the fire out at once (`bds_command_probe douse`, research/captures/bds-douse.txt): a
  `LevelEvent` `ParticleFizzEffect` from the low corner of the mob's box, its data the box's width and
  height in blocks begun (width | height << 8: a pig 257, a cow 513, a spider 258, an iron golem 770),
  and a tick later the sound `fizz` (-1) from the middle of the box. A mob lying dead is put out the same.
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

## Hits (2026-10-09, `bds_command_probe hit`, research/captures/bds-hit.txt)

Built in `sapling-shard/src/creatures/attacks.rs`. The actor's attack is an `InventoryTransaction`
`ItemUseOnEntity` `Attack`.

- A bare hand costs a cow 1, a wooden sword 5; a diamond sword killed one with 3 left. Every tool
  was then drilled (`tools`, bds-tools.txt): a sword costs 5 of wood or gold, 6 of stone or copper, 7 of iron,
  8 of diamond, 9 of netherite; an axe 1 less, a pickaxe or hoe 2 less, a shovel 3 less; a trident 9, a
  mace 6, a stick 1.
- The hit is told at once as `HurtAnimation` with cause 2 (`DeathAnimation` when it kills). With the next
  tick come the sound `attack.strong` from the middle of the hitter's box, naming `minecraft:player`, and
  the mob's health.
- Two hits sent together: one lands (the 10 tick guard), both are heard.
- The mob is thrown at 0.4 away from the hitter across and 0.4 up: a still cow 2 blocks east moved 0.4
  east and 0.4 up in the tick and was told a motion of 0.2184, 0.3136 (ground drag 0.546, then gravity).
  A walking pig kept some of its own speed; sapling keeps half of what the mob moved at, as the Java game.
  The motion BDS tells of a mob is what it will move by next tick; sapling tells what it last moved by.
- A mob a player killed leaves experience orbs when its body goes, 21 ticks after: a cow two orbs of 1.
  The amounts are the packs' `minecraft:experience_reward` (`mob-traits.tsv`, `xp=`); `Math.Random(1,3)`
  is taken as 1 or 2, by that one cow.
- Reach (`reach`, bds-reach.txt): a cow whose middle was 3.2 blocks from the hitter's was hurt, one 3.4
  off was not, in survival and in creative alike; from the eyes to the cow's box that is 2.77 and 2.97.
  A hit too far is answered with nothing, not even the sound. sapling allows 3 from the eyes to the box;
  what BDS measures was not told apart (one size of mob only).
- With the death BDS also sends two empty `MobEquipment` for the mob, a `mob.warning` sound from a cow
  and `SetEntityData` `TargetEid` 0; sapling sends none of them.

## The sun (2026-10-11, `bds_command_probe sun`, research/captures/bds-sun.txt)

Mobs summoned 4 blocks from the actor under an open sky:

- A zombie, a skeleton, a stray, a drowned, a zombie villager and a phantom were on fire the tick
  they came and lost 1 each second from that tick, as fire's own tick (`HurtAnimation` 7); the
  fire went out and was lit again 8 seconds in. A creeper, a husk and a spider were not.
- Under a roof of stone or of leaves 3 blocks up a zombie did not burn; under glass it did.
- It burnt at the times 0, 6000, 12000 and 23500 and not at 12600, 13000 or 23000, nor at noon
  in rain.

Sapling burns the kinds whose pack entry has `minecraft:burns_in_daylight` (`mob-traits.tsv`
`burns_in_sun`) so while the sky is darkened by 3 or less and their head's cell has sky light 15
(`creatures/harm.rs` `is_sunlit`); rain keeps them from it where it falls on their feet in a
biome too warm for snow. BDS's rule (`BurnsInDaylightSystem`, read with bdsre) is in
growth.md "Light": BDS also spares a mob with something on its head, which sapling's mobs never
have, and one in powder snow, which sapling does not ask.

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
the armour stand's own burning (bds-lava.txt: hurt animations with data -1), players. Of hits: critical hits, enchantments, an item's wear, the hitter's
hunger, a baby's experience, what a mob does once hit.
