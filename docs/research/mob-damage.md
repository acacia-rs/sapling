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
  stayed within z ±48 because `SAPLING_DRILL_WORLD=open`'s ground ends there. [U] whether the
  circle is `tick-distance` or a fixed 4 (the BDS copy runs 4), whether BDS ticks a hidden mob, and
  whether players and their packets about hidden mobs follow the same circle (sapling still sends
  a mob's moves to every view covering it).

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
  rains on mobs. Fixed: `blocks/growth/snowfall.rs` reads `weather::falls` (any rain). TODO: the
  odds by level and downfall (sapling: a cauldron fills at every pick, snow 1 in 16, BDS's odds at
  full rain over a biome of downfall 0.47 or more).
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
- Not drilled, from the Java game where bdsre was not read: bees stay home while rain is over 0.2
  (`!weather::rains`), monster spawns reckon brightness at the storm's 10 once rain times lightning
  is over 0.9 (`weather::storms`, `isThundering`), a night is skipped by the time window alone
  (`rest::is_night`, the sky's state choosing the rainy window).

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
