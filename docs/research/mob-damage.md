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
- Reach: see reach.md. bds-reach.txt does not measure it. Every hit there past about 3 blocks is
  `Err(NotPossible)` from acacia's own reach guard of the time, so it was never sent. A hit too far is answered
  with nothing, not even the sound.
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
- BDS also flips `MOVING` (bit 34) and another flag (bit 35) on mobs; sapling sends their recorded flags.

## Landings, roofs, fish (2026-10-29, `bds_command_probe harm2`, research/captures/bds-harm2.txt)

Sapling's run: sapling-harm2.txt (after the fixes below).

- 8 blocks onto stone cost a cow, a baby cow and a zombie 5; onto hay, slime or water nothing; 8.44 onto
  a bed 1. The actor in survival: 5 onto stone, nothing onto hay or water. Sapling counted every landing
  in full; it now counts the height times 0.2 on hay, 0 on slime and 0.4 on a bed (`creatures/heat.rs`
  `softens`; the bed's share fitted to that one drop, the Java game's half would cost 2).
- A roof one block up of stone smothered a cow (2 every 10 ticks, cause 4); glass, leaves and a bottom
  slab did not. Sapling the same.
- A cod, a salmon and a pufferfish on dry stone were hurt 2 (cause 4) 299 ticks after they came and
  died 10 ticks later (3 health). Sapling's fish breathed air; now a `no_air` kind out of water holds
  its breath and then smothers (`Health::gasp`). `/summon tropical_fish` fails on both.
- A cow shut in water drowned at the same ticks on both (2 every 10, cause 9) before the water was
  drained, so how breath comes back stays undrilled.
- The actor standing in fire: 1 every 10 ticks, then 7 burns on, 1 a second, on both.

## Fire, magma, rain (2026-10-29, `bds_command_probe burn2`, research/captures/bds-burn2.txt)

Sapling's run: sapling-burn2.txt.

- Soul fire cost a cow 2 a hit and a lit soul campfire 2, a campfire and fire 1, every 10 ticks, cause
  6; a campfire of either kind never set the cow alight (no `ONFIRE`). Magma under it cost 1 every 10
  ticks, cause 22. Sapling hurt only on fire and lava; now `creatures/heat.rs`.
- Out of fire after 1 s, a cow burnt on 160 ticks, 7 burns of 1 (cause 7), as sapling's; one out of
  lava after 1 s burnt on until it died.
- With fire resistance in lava a cow took nothing and was never shown `ONFIRE`; sapling showed it.
  Fixed: fire resistance lights no fire.
- A zombie and a baby zombie in lava took 0.94 of fire's 1 and 2.996 of lava's 4 (its own 2 armour);
  in full diamond 0.21. Sapling's mobs had no armour; the zombie family now has 2 (`birth.rs`
  `SKIN_ARMOUR`), and a player's sword goes through it as BDS's (7.872 of a diamond sword's 8,
  bds-hits2.txt). `replaceitem` armour on a mob is now taken and shown as BDS's
  (bds-fixreplace.txt, `creatures/command_gear.rs`): MobArmorEquipment and `armor.equip_<kind>`
  for a worn piece, MobEquipment alone for the main hand, `armor.equip_generic` for an offhand
  shield, stone on the feet taken unseen, saddle and inventory slots an exception. The armour is
  `creatures/worn.rs`'s, so it counts against damage.
- An armour stand in lava had 6 health and lost 0.05 every 10 ticks (cause -1); sapling's has none.
- A burning cow in rain went out 20 ticks after its last burn, the rain begun between: at the turn
  of its next burn. Sapling's burnt on; now `Health::rained_on` (tests/integration/mob_heat.rs). In
  sapling's drill_server rerun the cow still burnt on, as kinds3's endermen took no rain there.
  Settled 2026-11-03 (`bds_command_probe rainvoid`, research/captures/bds-rainvoid.txt): BDS's flat
  drill world is open to the sky, and its cow went out in the rain with the steam (`ParticleFizzEffect` at its
  corner) and a `fizz` a tick later; an enderman at y -60 was hurt (cause 9). Sapling's drill_server
  has a stone ground at y 60-63 over the drills, so no rain reached them. `SAPLING_DRILL_WORLD=open`
  drops it (sapling-rainvoid.txt): the cow went out, without the steam; fixed
  (`creatures/harm.rs`, tests/integration/mob_heat.rs). Its enderman teleported under the drill
  floor and took no rain, so that was not judged. The campfire and fire resistance cows matched.
  Re-drilled 2026-11-03 on open ground (`bds_command_probe enderrain`, bds-enderrain.txt: an
  enderman on the flat ground 60 east of the floor, the bots beside it): BDS's teleported now and
  then before the rain; ~1 s into it, it was hurt 1 (cause 9, no `splash`) and teleported seven
  times in 7 ticks, out of sight. Sapling's rain hurt it as BDS's, but rain did not drive it off
  (`teleports.rs` counted water alone); now rain on its feet or head counts, test
  `an_enderman_in_rain_teleports_on_and_on`. Re-drilled on sapling (sapling-enderrain.txt): it
  teleports every tick of the rain, hurt 1 (cause 9) every 10 ticks, as BDS's while in sight.
  Re-drilled 2026-11-04 after the rain ramp (TICK_SCALE 1, `SAPLING_DRILL_WORLD=open`): first hurt
  1005 ms after `/weather rain` (BDS 1009), teleports every tick from then, as BDS's. It stayed in
  sight, 1 to 3 chunks off: its walk is random (BDS's left at 3,-3 after 0.4 s of teleports), and
  sapling's ground ends at z ±48, where BDS's landed past. Same rule.
- Rain sets in (2026-11-03): BDS's first rain hurt came 1009 ms (20 ticks) after `/weather rain`,
  sapling's at once. `Weather::tick` (0x14119d160) moves the rain level 0.01 a tick towards its
  target (both clamped to 0..1), and `isRainingAt` (0x14119d6b0, reached from
  `ActorInWaterOrRainTest` through 0x140d2a130) wants it over 0.2: in f32 that is the 20th tick of
  rain, and rain still falls 80 ticks after `/weather clear`. bds-rainvoid.txt agrees: its burning
  cow burnt once more on the `/weather rain` tick and was put out, the enderman beside it first
  hurt, 1005 ms after. Fixed: `weather::rains` follows the
  level (`Falling`), which mob harm, the enderman's teleports and snow golems read; tests
  `rain_sets_in_over_20_ticks_and_clears_over_81`, `an_enderman_in_rain_teleports_on_and_on`.
  `/weather` still answers and tells clients at once, as BDS's did. The other rules of the sky:
  "What the rain's level drives" below.
- The enderman leaving sight (2026-11-03, `bds_command_probe farmobs`, bds-farmobs.txt): not its
  teleports, which are the same ±16 cube on both (mean step 8-10 blocks). BDS shows a mob only
  within a circle of `tick-distance` (4) chunks about the player: slowed mobs moved 3,2 and 4,0
  chunks off stayed shown, 3,3 and 5,0 off were removed (`RemoveEntity` the tick after), and
  `tp` found them and moved them back (`AddEntity` again), so the level kept them. The enderman's
  last teleport landed 3,3 chunks off. Sapling showed mobs as far as the client's chunk radius
  (8 here); now `mobs::update` takes `tick-distance` (test
  `a_mob_is_shown_only_within_the_tick_distance_circle_as_on_bds`). Sapling's drill enderman also
  stayed within z ±48 because `SAPLING_DRILL_WORLD=open`'s ground ends there. Re-drilled on
  sapling 2026-11-04 (sapling-farmobs.txt): the same mobs hidden (3,-3 and 5,0) and shown again on
  the same lines. Their heads: the fifth wave's `gaze` drill, below.
- Where a mob left to itself looks (2026-11-04, `bds_command_probe gaze`, research/captures/bds-gaze.txt;
  bds-tickcircle4.txt, bds-farmobs.txt): six cows slowed by slowness 255, three 3 blocks from the bots
  and three 20 off. `look_at_player` (a cow's: 6 blocks, 0.02 a tick, 2 to 4 s): the head turns 7 of
  the wire's 256ths a tick (10 degrees) until it points at the nearest player's eyes (56 and 108
  degrees, both exact), the pitch jumps at once to the eyes (251 or 252, about -6 degrees), and 53 to
  79 ticks later the pitch is 0 again in one tick while the head stays. The far cows' heads turned
  alone (`random_look_around`), their pitch never. With no look the head stays put; after the body
  turned (a stroll's step even at speed 0, 21 a tick) it came round to the body at the same 10
  degrees, but once it stayed. Fixed: `creatures/gaze.rs`, after the stroll, with each kind's
  `looks=`/`glances` from the packs (mob-traits.tsv); off with `mobs_stroll` as strolling is; test
  `a_slowed_cow_turns_its_head_and_pitches_at_a_player_near_it`. Re-drilled on sapling
  (sapling-gaze.txt): the near cows' heads come round 7 a tick to 40 with pitch 251, as BDS's; pitch
  changes 66 against BDS's 58, but 1846 moves against 592 (settled by the sixth wave, next). Left then: the random look's sides and length
  (Java's 20 to 40 ticks and a side within 90 degrees of the body are used, and look too often), and
  the body's turn rate (sapling's legs keep 60 a tick).
- Head, body and pitch per tick (2026-11-04, `bds_command_probe glance`, research/captures/bds-glance.txt):
  three slowed cows 3 blocks from the bots and three 9 off (outside `look_at_player`'s 6, inside the
  10.7 under which `conditional_bandwidth_optimization` sends every tick), 60 s. What fits:
  - A hidden body yaw (BDS's `MobBodyRotationComponent`, body control 0x142cc5810 per entity via
    0x148385570) under the head, which stays within 75 degrees of it. Standing, the body holds still
    while the head moves; once the head stays within 15 degrees of where it settled for 10 ticks the
    body closes on it, the allowed gap shrinking 7.5 degrees a tick (75 x (1 - (t - 10)/10)); walking
    (a tick's move across over 0.0005) it comes round to the legs' yaw 25 a tick. On the wire: a head
    held at 75 off the body waits 11 ticks, then creeps 5 to 7 of 256 a tick for three ticks, then
    waits again (cow 24 ticks 448-474, cow 20 ticks 519-540). The decompile has the head kept 85 off the
    body (look control 0x142cc51e0) and the body dragged to 75 at each 15-degree reset; the capture's
    stops are at 75 (53 of 256) and never drag, so sapling follows the capture.
  - `look_at_player`: the head 10 a tick (7 of 256) towards the eyes, within the 75; the pitch at once.
    `random_look_around`: the head put at once on a side, any of 360 degrees, held to the 75 (jumps of
    up to 53 in a tick). Neither: the head back to the body 10 a tick, so after a long look it stays
    (the body caught up) and after a cut-short one it comes back (cow 21 ticks 89-94).
  - A stroll's turn (`rot_y` 21 of 256, 30 degrees, a tick; the slowed cows' strolls go nowhere) ends a
    look in the tick it starts, pitch 0 with the first `rot_y`; the head is left where it was.
  - Looks start about half as often as their chance a tick: near cows' looks 80 to 280 ticks apart
    with 2 to 4 s looks, far cows' glances some 180 ticks apart. Fitted as a look tried every other tick
    and glances of 40 to 80 ticks.
  - A rotation of 2 of 256 summed over head, yaw and pitch is never sent alone, 3 is, on both servers
    (BDS's 4-degree test, research/bds-actor-move-sync.md; sapling's `sync.rs` `TURNED`).
  Fixed: `creatures/gaze.rs` keeps the head, the body and the settling count; strolls turn 30 a tick
  (`legs::STROLL_TURN`; hunting keeps 60, mob-attacks.md "Walking"); test
  `a_slowed_cow_left_alone_glances_at_once_and_strolls_round_30_degrees_a_tick`. Re-drilled on sapling
  (sapling-glance.txt): 507 moves to each bot against BDS's 495 (near cows 120/95/149 against 71/95/130,
  far 59/31/53 against 66/72/61), pitch changes 36 against 42. The fifth wave's `gaze` re-drilled
  (sapling-gaze.txt): 1002 moves against BDS's 592 (was 1846), far cows 20/46/46 against 26/40/33;
  the rest are the near cows walking in the seconds before `/effect` (208 moves with a position,
  BDS's 41): sapling's strolls start more often than BDS's (strolling.rs's fitted-by-eye TODO). Not drilled: an
  unslowed mob's head speed (BDS's look control turns at most 30 x an attribute's value over its
  default a tick, 0x142cc5323; which attribute, and what gives the slowed cows' 10, not pinned).
- The view circle against `tick-distance` (2026-11-04, `bds_command_probe tickcircle` on BDS set to
  4, 6, 8 and 10, research/captures/bds-tickcircle<N>.txt): 14 slowed mobs moved from beside the bots
  (chunk 3,0) to chunk offsets from 4,0 to 10,2, poisoned there and moved back 6 s later. At every
  `tick-distance` only the 4,0 mob stayed shown, so the view circle is a fixed 4 chunks, not
  `tick-distance` (corrected: the third wave took it for `tick-distance`). A hidden mob is still
  ticked within `tick-distance`: it came back poisoned (pig 3,3 off at 6: health 3 of 10), one past it
  came back whole and only then took the poison (10 of 10). Ticked, by offset: at 4 4,0; at 6 3,3,
  5,0 and 6,0, not 5,3, 5,4 or 7,0; at 8 5,3, 5,4 and 7,0, not 8,0, 7,4 or 6,6; at 10 up to 9,0, 8,0,
  7,4 and 6,5, not 6,6, 10,0 or 10,2. dx²+dz² <= tick-distance² fits all but four edge cells (5,3 at
  6, 8,0 at 8, 6,6 and 10,0 at 10). The region, read (2026-11-04): `LevelChunkTickingSystem::tick`
  (0x14661a960) runs 0x146629910 per player, which takes `Level::getTickingOffsets` (Level vtable
  +0x828, 0x1407db1b0: `ChunkTickOffsetManager` at Level+0x650) and ticks each offset's chunk from the
  player's; `ChunkTickOffsetManager::initialize` (0x1409e6bd0, called with the tick range from
  0x1407bd7c0) lists, for i in 0..r, the rows z = ±(r - i) with |x| <= i + 1, and the row z = 0 with
  |x| <= r: a diamond, |x| + |z| <= r + 1 with |x|, |z| <= r. It fits every mob above but the tips 8,0
  at 8 and 10,0 at 10, which are in the list yet did not tick; a chunk ticks only at load state 11
  or more (0x146629910 checks `+0xf0`), and those tips lie at the edge of what the bots had loaded
  (9,0 at 8 had unloaded): left as loading, not the region. `/fill` agrees with the list exactly
  (below). Fixed: `ticking::in_tick_region`, used by mobs (creatures/ticked.rs), spawning, snowfall
  and hives (`near_players`, before a square); tests `the_tick_region_is_bds_offset_list`, tick_circle.rs
  (5,3 at 6 now unticked).
  Mobs 9 and 10 chunks off at 4, 6 and 8 were not found (`noTargetMatch`) when moved back: the level
  had unloaded them. Fixed: `mobs::update` shows a fixed 4 (`SHOWN_WITHIN`), and
  `creatures/ticked.rs` leaves a mob outside every player's `tick-distance` circle (and not in a kept
  column) unmoved, unthinking and with its effects held, test tick_circle.rs. A world without
  players still ticks its mobs (BDS's would not), for headless shards and tests. Re-drilled on
  sapling at 4 and 6 (`SAPLING_TICK_DISTANCE`, sapling-tickcircle4/6.txt): the same mobs hidden, and
  the same ones ticked but 5,3 at 6 (an edge cell above). A returning mob's first poison hit lands
  before its AddEntity on sapling (health 9 of 10), just after it on BDS (10, then 9 the same tick).
  Mobs 9 and 10 chunks off stayed on sapling: the drill world's ground columns never unload.
- Commands past the tick region (2026-11-04, `bds_command_probe tickedge` at `tick-distance` 4,
  research/captures/bds-tickedge.txt, and the four tickcircle captures' pad fills): one-block fills
  at every chunk offset 0..6 by 0..6 and ten on the other sides answered `commands.fill.outOfWorld`
  exactly outside the diamond above, at 4, 6, 8 and 10. Into the first chunk past it `/setblock`
  answered `commands.setblock.outOfWorld`, `/testforblock` `commands.testforblock.outOfWorld`, `/clone`
  (from or to it) and `/testforblocks` `commands.generic.outOfWorld`; a fill of the last cell inside
  worked, one cell further reaching over did not (BDS's `CommandArea`, 0x141205fe0, tries the box
  grown by a block, then the box itself). Sapling filled any loaded column. Fixed: `ShardCtx::ticks_chunk`
  (a player's region or a kept column; a world nobody is in ticks all), checked by fill
  (`regions.rs`), clone and testforblocks (`copying.rs`), setblock and testforblock (`blocks.rs`); test
  world_commands.rs. Drilled on sapling: sapling-tickedge.txt, same answers.
- Fixed (fifth wave): a hidden mob's packets. BDS sent a client nothing of a mob it was not shown
  but a `/tp`'s forced `MoveEntityDelta` (bds-tickcircle6.txt: 10 per client, all teleports;
  bds-farmobs.txt), where sapling sent each covering view 72 moves and 41 health updates
  (sapling-tickcircle6.txt). `entities/tell.rs` hands a mob's packets only to clients that have it
  (`SeesMobs`), and `/tp` (`entities::teleport`) is a forced move told to every view covering the
  mob, as BDS's; test `a_mob_hidden_from_a_player_is_not_told_to_it_but_its_teleport_is`.
  Re-drilled on sapling (sapling-tickcircle6.txt, sapling-farmobs.txt): only forced teleport moves
  reach a client that was not shown the mob, 13 per client to BDS's 10 (sapling keeps the far mobs
  BDS unloaded, so more `tp`s find a target). farmobs' hidden watcher now gets the enderman's
  `tp` back as BDS's: forced, `x: Some(56.5)`, told in the command's tick even when the mob is
  outside every tick circle.

## What the rain's level drives (2026-11-04, `bds_command_probe rainramp`, research/captures/bds-rainramp.txt)

`Weather` (BDS's, per dimension at +0x1e8) holds rain at +0x34 (last tick), +0x38 (now), +0x3c (the
sky's 0 or 1) and lightning at +0x40/+0x44/+0x48; 0x14119d160 moves each 0.01 a tick towards its
target. Sapling: `weather/falling.rs` (`Falling`, both levels). What reads which:

- Rain on a block, `isPrecipitatingAt` (0x14119d6b0): rain now over 0.2, the biome precipitates, the
  cell at or over its column's rain height; rain rather than snow wants the temperature over 0.15
  (0x14119d830). Its callers: mobs in rain (`Actor::isInRain` 0x140d61650..0x140d61a90, `Mob` tick
  0x142a47470, `ActorInWaterOrRainTest`), fire (`FireBlock` tick 0x147157ed0, also its own 0.2 test),
  the fishing hook (`FishingHook::normalTick` 0x1429ca86f, at the cell over the hook), lightning's
  target (0x1409fdbc0). The drill: 49 fires laid on open stone one second after `/weather clear`
  (rain still 0.8) were all out within 5 s (`fill … air` answered 0), as 49 laid in dry weather all
  still burnt after 4 s (49); a riptide trident used and let go 2-3 s after clear threw the actor
  (`item.trident.riptide_1`), one 9 s after did nothing. Fixed: fire (`blocks/fire.rs`), fishing
  (`fishing/bite.rs`) read `weather::rains`; tridents' riptide, Impaling's wet target and conduits
  read `creatures::in_rain` (the feet's or head's cell rained on), where they had read the sky's
  state anywhere, roofed or not. Re-drilled (sapling-rainramp.txt, `SAPLING_DRILL_WORLD=open`):
  49 fires dry, 0 after the clear, one riptide throw 2-3 s after it and none later, as BDS. Its
  `/setblock` detectors read the ramp (15, 14, 13, 12 a second apart) where BDS's froze. Tests `a_riptide_throws_only_in_the_rain_still_falling` (shots),
  `fire_goes_out_with_the_block_under_it_and_in_rain_but_not_over_netherrack` (now 65 ticks).
- Bolts (`LevelChunk::tick` 0x1409f98a0, 0x1409fb2xx): rain now times lightning now over 0.9, then
  `nextInt(100000)` a chunk; the skeleton trap after. A storm from clear strikes from its 95th
  tick. Fixed: `lightning::thunders` (natural bolts, Channeling) reads `weather::storms`; test
  `bolts_wait_for_rain_and_lightning_both_near_full`. Undrillable here: one bolt in 100000 a chunk
  a tick shows none in seconds.
- Snow and cauldrons (`LevelChunk::tick`, after the freeze test): the rain's last-tick level times
  the biome's downfall (+0xc); over 0 it draws `nextInt(256 - min(512·that, 240))` and on 0 tries a
  snow layer (0x14119dec0, `doWeatherCycle` on) and the column top's block's precipitation (a
  cauldron's fill). So they fall from the rain's first tick until ~100 ticks after it clears; BDS's
  cauldron took its first fill 863 ms after `/weather rain` (bds-cauldronrain.txt), before rain
  rains on mobs. Fixed: `blocks/growth/snowfall.rs` reads `weather::falls` (any rain). The odds
  (2026-11-04, `bds_command_probe cauldronrate`, research/captures/bds-cauldronrate.txt): the
  downfall the tick weighs is the rain's level times the biome's (`BiomeDefinitionList`'s, which
  sapling-data now keeps: `biome_downfall`, plains 0.4); the tick calls the top block's
  `handlePrecipitation` with it on every pick (BlockType vtable +0x170), and only the cauldron's
  overrides it (141a46850): it fills unless the downfall is under `nextFloat() * 0.5`, so twice the
  downfall, sure from 0.5. A chunk of 256 cauldrons on plains at `randomTickSpeed` 1 filled 16 a
  second in full rain (0.8 a pick) and 34 more over the 5 s the rain cleared; sapling's 20 a second
  and 99 (every pick). The tick loops `randomTickSpeed` times a chunk with no cap (bds-cauldronrain.txt
  at 100: six fills in 3.1 s of rising rain, as 100 picks predict; sapling had capped at 16). Fixed:
  snow lies at `nextInt(256 - min(512 x downfall, 240)) == 0` and a cauldron fills at twice the
  downfall, `randomTickSpeed` picks a chunk (tests `snow_lies_one_pick_in_sixteen_from_downfall_047_and_more_rarely_under`,
  `rain_fills_a_cauldron_at_twice_its_downfall`, `biomes_carry_bds_downfall`). Re-drilled
  (sapling-cauldronrate.txt): 345 fills in 20 s of full rain (BDS 335) and 38 in the clearing (34).
  Snow by downfall is undrillable here: the flat drill world is
  plains (too warm to snow) and BDS has no command to set a biome.
- The sky's darkening (`Dimension` tick 0x14187f9c0): `(1 - rain·5/16)·(1 - lightning·5/16)`, each
  level now, lightning not weighed by the rain. bds-daylight.txt's noon dust read 14 some 31 ticks
  into the rain and 13 some 61 in (this gives 15 up to rain 0.29, 14, 13 from 0.58), 12, 11, 10 at
  7, 27 and 69 ticks into the thunder that followed, then 11 and 12 as it cleared. Fixed:
  `light::sky_darken` takes `Falling`: daylight detectors, monster light, bees' night, a bed's
  `skyDarken >= 4` (`sleep/rest.rs`: sleep by day only once a storm darkens noon to 4, ~65 ticks
  in; test `a_monster_near_keeps_a_sleeper_up_and_a_storm_lets_it_sleep_by_day`). Tests
  `the_sky_darkens_as_the_rain_and_lightning_fall_harder`. Detectors set by `/setblock` in the drill
  read once and froze, as redstone.md says, so bds-rainramp.txt's two show nothing of the weather.
- What players are told (`LevelEvent` StartRain/StopRain/StartThunder/StopThunder) follows the
  sky's state at once, no level on the way: bds-weather.txt, bds-rainramp.txt. Same on sapling.
- Bees (2026-11-04, `beerain`, research/captures/bds-beerain.txt against sapling-beerain.txt):
  three bees by a hive at noon, no flower near. Rain sent them in 4.2, 4.6 and 5.2 s after
  `/weather rain` on BDS, 3.2, 3.9 and 4.6 s on sapling (rain over 0.2 from 1 s, then the flight:
  the same within the flights' spread). After 35 s of rain, `/weather clear` let them out 84, 85
  and 96 ticks on (BDS), 81 for all three (sapling): rain under 0.2 at 81 ticks, the hive's check
  a few ticks apart on BDS. Drilled the same. The pack's `is_daytime` filter (`FilterTestDaytime`
  0x1411bbd80) is the dimension's day test and `skyDarken` under 4, which `may_fly` reads.
- Sleep and the night's skip in rain (`rainnight`, bds-rainnight.txt against sapling-rainnight.txt):
  at 12200 BDS refused a sleeper in clear sky and with rain begun 2 s before (level 0.4), took it in
  full rain and skipped the night 100 ticks on (12390, short of the clear window's 12541), woke a
  sleeper without a skip as a clearing rain fell under 0.52 (`skyDarken` 3 at 12318), and skipped
  at 23937 in full rain and from 18000 clear, each with `StopRain` where it rained. Sapling took
  the sleeper 2 s into the rain: `rest::is_resting_time` read the sky's state's window. Fixed: whether
  one may sleep, and stay asleep, is `skyDarken >= 4` of the levels alone; the skip keeps the
  state's window, which every skip above fits (test
  `rain_lets_one_sleep_early_only_once_it_darkens_the_sky`). Re-drilled (sapling-rainnight.txt):
  refused, refused, slept and skipped, woken in the clearing, skipped, skipped, as BDS.
- The skip's packets (2026-11-04, bds-rainnight.txt). BDS sends WakeUp, then `StopRain` and
  `StopThunder` (thunder too, where only rain fell), then the clock. It sends no sleeper count
  (LevelEventGeneric 19602) at the skip's wake. A dry skip sends WakeUp and the clock alone.
  Sapling sent WakeUp, the count, the clock, then `StopRain`. Fixed: `sleep::tick` wakes without
  the count and stops the weather before the clock, and `weather::stop` adds `StopThunder` for
  those told rain. Test `a_night_slept_through_is_skipped`.
- Not drilled: monster spawns reckon brightness at the storm's 10 once rain times lightning is over
  0.9 (`weather::storms`, `isThundering`, the Java game's).

## Hits (2026-10-29, `bds_command_probe hits2`, research/captures/bds-hits2.txt)

Sapling's run: sapling-hits2.txt. Each mob held by slowness 255, the actor put back on its mark.

- A diamond sword cost a cow 8, a spider 8, a zombie 7.872 (its 2 armour); Sharpness I 9, V 14;
  Smite II a zombie 12.792; Bane of Arthropods II a spider 13. A zombie in a diamond chestplate took
  5.824. Knockback I with an iron sword 7, a wooden sword 5.
- Fire Aspect I burnt a cow a second apart after the iron sword's 7 (cause 7).
- A hit while the actor jumped (`!jump`) cost a cow 12, the sword's 8 times 1.5.
- A hit cow ran off out of reach (3.6 blocks), so the sword's wear and repeated hits were not seen.
- `effect` health_boost raised BDS's cows to 30 health; sapling's stay at 10 (fixed, see "Health boost").
- Sapling's run matched each sword, enchantment and the zombie's 7.872, but not the jump: its cow lost
  8, no critical hit (not followed), and its chestplate zombie took 7.872, the chestplate refused.

## Health boost and armour on mobs (2026-10-30, `bds_command_probe boosts`, research/captures/bds-boosts.txt)

- Health boost on a cow: told a tick after the command as a player's is (`UpdateAttributes` health, max 10 +
  4 a level, `default_max` 10, the `HealthBoost` modifier); it does not heal. Healed it shows 30 of 30; a
  weaker boost over it changes nothing; `effect clear` or running out drops the most back, the health
  clamped to it (25 to 10; a zombie's 20 of 24 to 20). Fixed: `creatures/boosts.rs`, shared with players
  in `vitals/boost.rs` (mob_commands.rs).
- `replaceitem ... slot.armor.chest` on a zombie: `MobArmorEquipment` at once. `/damage` 8 (no cause)
  then cost it 5.824, armour and its own 2 points as a sword's hit. Fixed: a mob wears armour
  (`creatures/worn.rs`, `ShardCtx::set_entity_armour`, and a world folder's `Armor`), and `/damage`
  of a cause armour stops goes through it. `/replaceitem` on a mob now dresses it through `worn.rs`.
- Sapling's run (sapling-boosts.txt): every health line the same as BDS's; its zombie lost 7.872 to
  `/damage`, as its `/replaceitem` could not yet dress a mob (before fix1; not re-drilled).
- The actor's `!hit` fell 8.5 blocks short (the held cow and zombie had been pushed); the sword's
  5.824 stands from bds-hits2.txt.

## Not drilled

How breath comes back out of water; players' burns beyond fire; a baby's experience counted; the
hitter's hunger; an item's wear (the cow fled); what a mob does once hit beyond running.
