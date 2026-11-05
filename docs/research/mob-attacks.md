# Mobs that hit back

What BDS does when a zombie and a survival player share a floor, from `bds_command_probe bite`,
`slain` and `chase` (`research/captures/bds-bite.txt`, `bds-slain.txt`, `bds-chase.txt`; sapling's
are beside them), and the
zombie's definition in the vanilla pack (`entities.brarchive`, `zombie.json`). Easy difficulty.

## Picking a player

- The definition: `nearest_attackable_target` with `max_dist` 35 for players, `must_see`,
  `within_radius` 25; `melee_box_attack`; `attack` 3; `movement` 0.23.
- A zombie summoned 8 or 30 blocks from the player took it as its target 1 to 9 ticks later: it
  looks about every 10 ticks. One 40 blocks off never did.
- It tells its viewers by `SetEntityData` `TargetEid` (the player's unique id, 0 when it lets go)
  and, a tick later, the sound `mob.warning` at 0.85 of its height.
- A player in creative is not taken; turned to survival it is, within the 10 ticks. Turned to
  creative, or dead, it is let go a tick later.
- A wall two high between them, across the whole floor or five wide, and it took no target; a
  step one high did not hide the player.
- Glass hides a player as stone does (`around`, research/captures/bds-around.txt): behind a
  glass wall five wide the zombie strolled until it was past the wall's end, then took its target
  and walked straight at it; in a glass pen it took none.
- With no target the zombie looks around and strolls, at the same speed.

## Walking

- It turns first: its yaw went 0, -30, -90, -149, 180 in four ticks, its head more slowly.
- Each tick its speed across is `0.546 x speed + push`, along its yaw. The push grows over its
  first ten ticks as `0.98 x (0.23 x n/10)^2` and stays `0.98 x 0.23^2 = 0.05184`, so it ends at
  0.11419 blocks a tick.
- The turn carries it up to 0.009 off the line; it is back on it within a block.
- It walks on at full speed through its own hits.
- Another player in its way stops it: the watcher's box held it 0.55 off for seven seconds.
- The actor was sent every move; the watcher, 20 blocks further, every other one.

## A block in the way (`chase`)

- With the face of a block one high 0.46 from its middle (0.57 the tick before) it jumps: up 0.42
  in that tick, then `(v - 0.08) x 0.98` as anything falls, 1.25 at its highest.
- In the air its push is `0.02 x 0.98 x 0.23 = 0.0045` a tick and the drag across is 0.91.
- The drag of a tick goes by whether it stood on the ground before that tick's move: the tick
  after it lands is still slowed by 0.91, the tick after it walks off an edge by 0.546.
- `SetEntityMotion` goes out as its speed changes by a lot, holding the speed after the tick's
  drag.

## Around a fence (`around`)

A fence does not hide the player and is not jumped (`research/captures/bds-around.txt`, its
last two zombies):

- With a fence five wide 4 blocks before the player, the zombie took its target at once, walked
  to the middle of the cell past the fence's end, then cornerwise at the player, and hit it
  4.4 s after it was summoned.
- In a pen of fences open at the back it walked out of the back, round the end of the side
  fence, down the outside along the middles of the cells, and hit 6.8 s after.

## Hitting

- It hits in the first tick in which its box, 0.8 wider on each side, overlaps the player's,
  measured after that tick's move (1.4 between their middles), and again 20 ticks later while that
  holds.
- In the hit's tick: `EntityEvent ArmSwing` of the zombie, `SetEntityMotion` of the player and the
  player's `HurtAnimation` with 2 (entity attack); its health follows as after any hit. A tick
  later the sound `attack` at half the zombie's height, where it stood as it hit.
- 2.5 of health on easy: `3 / 2 + 1`.
- The player is thrown at 0.4 away from the zombie across and 0.4 up. The bot landed 2.07 blocks
  on, so a zombie that keeps hitting does so every 20 ticks.
- Being hit costs no exhaustion.
- The hit that kills throws nobody and flashes nothing. The death reads `death.attack.mob` with
  the player's name and `%entity.zombie.name`, in `DeathInfo` and to the world.
- `/difficulty easy` answers `commands.difficulty.success` with `EASY`.

## Other kinds (`kinds`)

Seven more on the same floor at midnight (`research/captures/bds-kinds.txt`). All take a target,
walk and hit as the zombie does, with their own numbers:

| kind | movement | blocks a tick | hit on easy | on a hit |
| --- | --- | --- | --- | --- |
| husk | 0.23 | 0.11419 | 2.5 | hunger, 600 ticks (`Update` on each later hit) |
| zombie villager | 0.23 | 0.11419 | 2.5 | |
| spider | 0.3 | 0.19427 | 2 | |
| cave spider | 0.3 | 0.19422 | 2 | poison for 0 ticks, added and removed |
| silverfish | 0.25 | 0.13491 | 1 | |
| vindicator | 0.35 | 0.26443 | 7.5 | |
| wither skeleton | 0.25 x 1.25 | 0.21080 | 5.5 | wither, 200 ticks |

- The speed is `0.98 x (movement x speed_multiplier)^2 / 0.454` for each: the push is the square
  of the speed, and the attack's `speed_multiplier` is inside it.
- The reach is the box rule with the kind's own width: a spider (1.4 wide) first hit from 1.8.
- The hit on easy is `min(attack, attack / 2 + 1)`, where `attack` adds what the mob holds: a
  vindicator's 8 and its iron axe's 5, a wither skeleton's 4 and its stone sword's 5.
- A silverfish 8 blocks off (its `max_dist`) took no target; 4 off it did.
- `/summon zombie_villager_v2` is a syntax error; `zombie_villager` summons one.
- The sounds come from the same shares of each kind's height.

## Creepers (`boom`)

A creeper 6 blocks from the actor on stone over dirt (`research/captures/bds-boom.txt`), with
`mobgriefing` on and then off:

- It takes its target and walks as the others do, at 0.1349 a tick (movement 0.2, its attack's
  `speed_multiplier` 1.25).
- 3.08 blocks off (2.48 between their boxes; its `target_nearby_sensor` has `inside_range` 2.5
  and `outside_range` 6, where it stops) it is lit: its flags gain `IGNITED` (1024) and `FuseLength` is sent as 29, then
  one less each tick down to 0; two ticks in, the sound `fuse` at its head. It walks on a few
  ticks more slowly and stands, 2.92 off.
- The tick after `FuseLength` 0: `LevelEvent` `ParticleExplosion` at its feet with data 3 (its
  power), the player's `HurtAnimation` with 11 and `SetEntityMotion`, `RemoveEntity` of the
  creeper, and a tick later the sound `explode` there, of no entity.
- The player lost 8.229 on easy: the Java game's `(i^2 + i) / 2 x 7 x 2 x power + 1` with
  `i = 1 - distance / (2 x power)` and the distance from the creeper's feet to the player's eyes
  (3.34), halved and one added for easy.
- It was thrown at (0, 0.615, 0.788): a hit's 0.4 away and 0.4 up, plus the way from the blast to
  its eyes times `i` (0.443).
- One block broke, the stone under the creeper; nothing lay there after. With `mobgriefing` off
  none broke and the player lost the same.

## Skeletons (`shoot`)

A skeleton 10 and then 4 blocks from the actor (`research/captures/bds-shoot.txt`):

- It takes its target, walks 1.7 blocks nearer in either case, and stands.
- Its first arrow comes 61 ticks after the target, the next every 61 (easy).
- The arrow is an entity `minecraft:arrow` added a block before the skeleton and 1.61 above its
  feet, with a speed of 1.4 to 1.7 a tick, mostly towards the player and a little off; the sound
  `bow` at the skeleton's middle.
- It keeps 0.99 of its speed a tick and falls 0.05 faster: `SetEntityMotion` (0.0178, 0.1118,
  1.6879) the tick after (0.0179, 0.1635, 1.7049).
- From 10 blocks all three flew over the player and stuck in the floor 13 blocks on: speed 0,
  `EntityEvent` `ArrowShake` with 7 and the sound `bow.hit` where it stuck. They stay.
- From 4 blocks it hit: `HurtAnimation` with 3 (projectile), the hit's throw of 0.4 away and up,
  4 of health (its speed of 1.72 twice, rounded up), `bow.hit`, and the arrow removed.

## The iron golem (`golem`)

The actor, floating down from each throw and healed between, hit a golem and took twenty of its
hits (`research/captures/bds-golem.txt`): on easy 4.5 to 11.5 in halves, which are whole hits
of 7 to 21; thrown away at 0.52 and up at 0.4288 (less where it was still in the air), 1.3 to
2.3 seconds apart as it walked after the actor at 0.049 a tick.

## Other archers (`shoot2`)

A stray, a bogged and a pillager 5 blocks from the actor (`research/captures/bds-shoot2.txt`):

- The stray did as the skeleton: 1.7 blocks nearer, an arrow 61 ticks after its target and each
  61 after. A hit cost 3 or 4 and gave slowness (2) at its first level for 600 ticks.
- The bogged shot 72 ticks after its target and 71 after that; a hit cost 3 and gave poison (19)
  for 112 ticks.
- The pillager walked on at the actor, to 1.7 blocks from it at its first shot 47 ticks after its
  target, and shot each 46 ticks; a hit cost 3 or 4. Before a shot the sounds
  `crossbow.loading.start`, `.middle` and `.end`, 10 ticks apart, the first 3 ticks after the
  shot before; it held `MobEquipment` anew at each.

## More kinds (`kinds2`)

Twelve more kinds 8 blocks from the actor at midnight on easy
(`research/captures/bds-kinds2.txt`):

| kind | a tick's step | movement x multiplier | each hit took | so its attack | between hits |
| --- | --- | --- | --- | --- | --- |
| drowned | 0.114 | 0.23 | 2.5 | 3 | 1 s |
| endermite | 0.135 | 0.25 | 2 | 2 | 1 s |
| piglin brute | 0.264 | 0.35 | 6 | 10 | 1 s |
| ravager | 0.345 | 0.4 | 7 | 12 | 1.55 s |
| hoglin | 0.280 | 0.36 | 1 | ? | 0.76 s |
| zoglin | 0.178 | 0.29 | 2.6 to 4.5 | varies | 1 s |

A slime and a magma cube hop and split, a polar bear hit without a target being told, a witch
threw potions, a blaze shot three small fireballs a second apart and an enderman was gone. The
first four of the table are built (`mob-traits.py`), and of the six the polar bear, the witch and
the blaze and the enderman (below); the slimes, the hoglin and the zoglin are not.

## The blaze (`blaze`)

A blaze 8 and then 20 blocks from an actor proof against fire
(`research/captures/bds-blaze.txt`), and one in `kinds2`:

- It took the actor from 20 blocks, went 2.5 blocks nearer and stayed; later it rose 3 blocks.
- 83 ticks after its target it shot three `minecraft:small_fireball`, 6 ticks apart, with the
  sound `shoot`; the next three came 173 ticks after the first of the last.
- A fireball is added where the blaze is, 1.4 above its feet, at 1.15 to 1.42 a tick towards
  the actor, and keeps that speed; it was gone within a second and a half.
- In `kinds2` one hit: `HurtAnimation` 3, 5 of health, the hit's throw, and the actor burning
  (`ONFIRE`) for five seconds, losing 1 four times. Proof against fire, the actor lost nothing
  to any of nine.

Sapling's blaze does so (`creatures/arrows.rs` `FIREBALL`); it walks and does not fly, and its
fireball lights no block.

## Spiders by day (`spiders`)

A spider and a cave spider 5 blocks from the actor in survival
(`research/captures/bds-spiders.txt`): at noon under the sky neither took a target in six
seconds; at noon under a stone roof and at midnight both did. Sapling's take none while the sun
is on them (`hunting::pick`, `harm::is_sunlit`); one that has its prey keeps it into the day.

## Hit back (`provoke`)

A wolf, a zombified piglin and an iron golem 1.5 blocks from the actor in survival, left alone
four seconds and then hit with the fist (`research/captures/bds-provoke.txt`):

- The wolf's bites came 1.55 seconds apart in both runs (31 ticks), where every other kind's
  hits came a second apart; sapling keeps that for the wolf (`every=31`).
- None took a target before the hit. Each took the actor the tick it was hit (`TargetEid`), with
  the `mob.warning` sound a tick later as a zombie's.
- The wolf bit for 2.5 on easy (3), the piglin hit for 5.5 (9: its 5 and its golden sword) once
  a second at a zombie's speed (0.114 a tick), and the golem for 10.5 and threw the actor up at
  0.43; its steps were 0.048 a tick.
- A second run, the actor gone to each mob before its hit: a bee hit by day took no target and
  flew on at 0.155 a tick; an enderman hit took none and was 9 blocks off the next tick. The
  piglin, hit 40 blocks under the floor where the enderman had led the actor, took none either,
  which the first run contradicts and no third settled.
- Run against sapling, once its `/tp` went to a mob as BDS's does, the wolf took its hitter and
  bit for 2.5; the piglin did as in the first
  run (5.5 a second apart, 0.114 a tick). BDS's first hit back came 1 to 1.5 seconds after the
  hit, and sapling's waits a second.

## Hit back by day (`provoke2`)

A polar bear, a panda, a spider, a cave spider, a llama, a piglin and a bee, as `provoke` at noon
under the sky (`research/captures/bds-provoke2.txt`, `bds-provoke2-findings.md`):

- The polar bear and both spiders took the actor the tick they were hit, with `mob.warning` a
  tick later. The bear hit for 4 on easy (its 6) every 39 ticks, the first 2.5 seconds after,
  standing still for a moment and rearing before each; it stepped 0.1349 a tick (movement 0.25).
  The spider bit for 2 every 20 ticks from 1.5 seconds after, at 0.194 a tick; the cave spider
  for 2, its poison added and taken away a tick later. Each hit threw the actor 0.4 away and up.
- The llama charged a second after its hit and spat 2.07 seconds after: a `llama_spit` 1.58
  above its feet at some 1.5 a tick, falling 0.06 a tick, for 1 and no throw (`HurtAnimation`
  data 3). It stayed where it was and let go of the actor 3.95 seconds after.
- The piglin took the actor 0.36 seconds after it was summoned, unhit (no gold worn), and hit
  for 5.5 (its 5 and its golden sword) every 20 ticks, its arm swung twice a hit.
- The panda rolled and the bee flew out of reach; neither was hit, so neither says anything.

`provoke3` (`bds-provoke3.txt`, `bds-provoke3-findings.md`) penned three pandas and three bees
with the actor in glass, so that each was hit a second after it came; two bees still flew out
of reach.

- Each panda, of the playful, normal and lazy genes, took the actor as it was hit, stood some 48
  ticks after its throw, walked up at 0.0485 a tick (movement 0.15; the lazy one at 0.0106, its
  0.07), hit once for 2 and let go of the actor in that tick: 74, 76 and 162 ticks after.
- The bee took the actor as it was hit and stung it 25 ticks after, for 2 and the throw of a
  hit, no poison on easy; that tick it let go and lost its sting (`MarkVariant` 1). It was alive
  and wandering 8 seconds on.

## Hoglins, zoglins, the warden and the creaking (`beasts`, `creaking`)

`research/captures/bds-wander-findings.md`: a hoglin hits every 40 ticks for 3 to 9 and a zoglin
every 20 for 3 to 8, drawn with fractions; both are heard `angry` a tick after their growl. A
warden hunts only a player that hit it, for 30 every 20 ticks, and no hit moves it. A creaking
takes a player that looks at it, stands frozen while anyone does, and hunts it at 0.4 once looked
away from, hitting for 3 every 40 ticks.

## Guardians (`guardians`, `guardianhits`)

A guardian and an elder guardian in a pool beside a survival actor on normal
(`research/captures/bds-guardians.txt`, `bds-guardianhits.txt`):

- It takes the actor (`TargetEid`) a tick after it is summoned and is heard `mob.warning` a tick
  later; 9 ticks on it shows its laser (`EntityEvent` `GuardianAttack`).
- 80 ticks after that (an elder 60) the laser lands: `HurtAnimation` 2 then 14 (magic) the same
  tick, health 20 to 15 from either kind, the throw 0.4 up and 0.4 to 0.6 away. Then `TargetEid`
  goes to 0 and it takes the actor again some 9 ticks later.
- An elder curses a survival player within 50 every 100 ticks: `MobEffect` mining fatigue (4) at
  amplifier 2 for 6000 ticks with particles, `EntityEvent` `ElderGuardianCurse` on the player.
  bdsre's `GuardianPreAIStepSystem` (1459a1130) skips one with amplifier 2 or more and 1200 ticks
  or more left; sapling follows it (`creatures/guardians.rs`).
- A player whose guard took the laser's bite whole still flashed the magic (`HurtAnimation` 14);
  sapling flashed nothing, now the same (`vitals::flash_guarded`).
- In `t2water` (research/captures/bds-t2water.txt) a guardian took an axolotl unprovoked, as its
  pack's `nearest_attackable_target` names squid and axolotls; sapling's took no mob. Now it takes
  either (`preys=squid,axolotl` in `mob-traits.tsv`) and its laser lands on a mob as on a player,
  then the magic (test `a_guardian_takes_a_squid_and_lands_its_laser_on_it`; sapling-t2water.txt's
  guardian took the squid, BDS's pool drained and the run was not alike enough to compare further).

## Mobs that fight their own way

The witch, the enderman, the goat, the evoker, the shulker, the breeze and the ghast: `mob-kinds.md`.

## Strolling (`stroll`)

A zombie and a cow left alone for a minute on a floor 40 blocks square, the actor in creative
(`research/captures/bds-stroll.txt`):

- Each walked in straight lines, 1 to 10 blocks at a time, six or seven times in the minute, and
  stood between for 1 to 15 seconds.
- The zombie strolled at 0.11419 a tick, the speed it hunts at. The cow went 0.08634: its
  movement 0.25 times its `random_stroll`'s `speed_multiplier` 0.8, by the same rule.
- Each walk ends with one more move of about 0.006.
- A chicken strolled at 0.135 (movement 0.25).

How often (2026-11-04, seventh wave, `bds_command_probe strolls` and `strollstart`,
research/captures/bds-strolls.txt, bds-strollstart.txt; tools: walks of 1 block or more, pauses under
1 s inside a walk): unslowed cows, sheep, pigs and zombies (by night) on a stone floor 25 across walled
three high. A stood mob sets off about a tick in 120 (`random_stroll`'s default `interval`): cows
once in 114 idle ticks over 91 walks, pigs 102 over 48 (strolls: 118 to 152 a kind), and the first
walk after a summon comes as soon (cows 0.2 to 23 s, mean 6 s, over 32). A walk goes 7.0 to 8.4
blocks on average (uniform places within 10 along each axis give 7.65) and takes 108 to 174 ticks,
some 80 of them pauses of 2 to 20 ticks on the way. Corrected: the sixth wave's "sapling's strolls
start more often" came from the seconds before `/effect` in `gaze`; on these floors sapling's started
less often (cows 1 in 157 idle ticks, pigs 132), because it tried one place a start and a place off
the floor or behind the wall meant no stroll, where BDS (as Java's `RandomPos`, ten tries) finds one.
Fixed: `creatures/strolling.rs` tries ten places (`TRIES`); test
`cows_in_the_corners_of_a_small_floor_still_set_off_about_a_tick_in_120`. Re-drilled on sapling
(sapling-strollstart.txt, sapling-strolls.txt): cows 1 in 103, pigs 99, zombies 116, sheep 140. Still
different: sapling's walks are 5.9 to 7.7 blocks and 79 to 104 ticks, with fewer pauses, as it walks
straight at its place where BDS follows a path (the TODO in strolling.rs).
Paths (2026-11-05, thirteenth3m): BDS's walks go cell middle to cell middle of a grid path, zigzagging up
to 4.5 off the straight line, each middle left at about 0.5 from it. Sapling now routes a stroll over
cells (`paths::route`) and walks middle to middle (`creatures/strolling.rs`; test
`a_strolling_cow_goes_round_a_wall_to_a_place_behind_it`). Re-drilled (research/captures/sapling-m3strolls.txt,
`walks.py`, cows, pigs, sheep): path 8.1, 7.7, 8.9 blocks against BDS's 7.8, 8.6, 10.5 (before 6.5, 8.3,
6.2) and 107, 74, 118 ticks against 116, 90, 152. Still different: in-walk pauses, 2 to 5 ticks a walk
against BDS's 10 to 12; their cause is not known (not reached in thirteenth3o).
Where they fall (2026-11-05, thirteenth3y, bds-strolls.txt): a walk opens with still ticks and then
10 ticks of pace building; inside it, pauses of 2 or more ticks come at the path's corners, after turns
of 50 to 180 degrees (pigs and sheep sometimes at small ones); it ends slowing to 0.022 a tick and
creeping to its cell's edge. So BDS stops to turn at a sharp corner where sapling turns as it walks;
not followed.

## Fleeing (`flee`)

A cow and a pig, each hit once by the actor in creative (`research/captures/bds-flee.txt`; the
chicken and the zombie had strolled out of reach):

- The hit throws it as measured before: 0.4 in its first tick, 0.218, then nine ticks in the air.
- Landed, it turns and runs: its steps grow over some fifteen ticks to 0.2108, which is
  `0.98 x (0.25 x 1.25)^2 / 0.454`, its movement times its `panic`'s `speed_multiplier`.
- The cow ran one stretch and stood 60 ticks after the hit; the pig ran three, turning between,
  and stood after 80. Then each strolled again at its own pace.

## Fights (2026-10-29, `bds_command_probe fights`, research/captures/bds-fights.txt)

Sapling's run: sapling-fights.txt. The actor under resistance on a stone floor, healed each second.

- A zombie hit 3 on normal, a second apart, as sapling's. On hard one hit 4.5, 5.5 s after it came;
  sapling's hard zombie took the actor and never hit it (not settled).
- A baby zombie hit 2.5 on easy, its first some 4.5 s after it came; sapling's 7 blocks off took no
  target in the run (not settled).
- A spider at noon and at midnight under the open sky took no target in either run; sapling the same.
- A cave spider took the actor 3 s after it came, dropped it 1.6 s later and never hit, so its poison
  was not seen. Sapling's hunted at once and hit 2 a second (differs, not followed: what made BDS's
  let go is not known). Re-drilled 2026-11-04 (`e1cave`, bds-e1cave.txt, on normal): by midnight one
  bit the survival actor about once a second for 10 s, each bite giving poison 140 (7 s), and never let
  go; at noon one took no target in 20 s. Sapling's (sapling-e1cave.txt) hunted and bit the same but
  gave no poison. Fixed: `mob-traits.tsv` `hits=poison:140`, test
  `a_cave_spider_s_bite_poisons_for_seven_seconds`. Easy and hard drilled 2026-11-04 (`t12cave`,
  `t12bites`, bds-t12cave.txt, bds-t12bites.txt): on easy each bite sent poison `Add` with duration 0
  and its `Remove` a tick later; on hard poison 300 (15 s), each later bite an `Update` to 300. A
  wither skeleton's wither on easy was 200, as on normal. Fixed: `hits=poison:140:0:300` in
  mob-traits.tsv (easy and hard after normal), `hunting/strike.rs`; test
  `a_cave_spider_s_bite_poisons_for_seven_seconds_none_on_easy_and_fifteen_on_hard`. In t12cave the
  hard spider took no target in 12 s and the easy one bit three times, so a bite is not certain.
  Sapling's re-drill (sapling-t12bites.txt): poison 300 then `Update`s to 300 on hard, wither 200
  on easy, as BDS's. Left: BDS sent each wither hit's effect twice in one tick (`Add` then `Update`,
  later two `Update`s). Fixed (twelfth wave): `gives_twice` in mob-traits.tsv makes
  `hunting/strike.rs` afflict twice; test `a_wither_skeleton_s_hit_gives_wither_twice_in_its_tick`.
- A creeper with its prey 6 off was lit and went out; one with prey 3.5 off burst; one whose prey
  went creative went out. Sapling the same.
- `/summon creeper ... minecraft:start_exploding_forced`: lit 3 ticks after, burst 30 after that with no
  prey, and hurt the actor 1 behind an obsidian wall. Sapling's had no fuse; now `blast::light_forced`
  (the rerun: lit 1 tick after, burst 29 after that, the actor hurt behind the wall).
- A skeleton's first arrow struck a cow standing between it and the actor (4). Sapling's arrows rose
  more steeply (0.19 to 0.25 against BDS's -0.15 to 0.28) and passed over the cow: not settled.
- 7 off behind a glass wall a zombie took the actor 3.65 s after it came; behind oak leaves none did;
  in the open one took it at once. Sapling's took it only in the open (glass not followed).
- A zombie the actor hit took and dropped it every 10 ticks for 4.4 s; sapling's took nothing (not
  followed).

## Fights again (2026-10-30, `bds_command_probe targets2`, research/captures/bds-targets2.txt)

The actor in survival at midnight on a walled stone floor, healed and put back between steps.

- Hard: a zombie 3 off took the actor 0.45 s after it came and hit every second from 1.5 s. Easy: a
  baby zombie 7 off took it 0.35 s after and hit every second from 2.4 s. The fights.txt misses
  were the actor falling off its floor, not the mobs.
- Four cave spiders held 5 off on normal took the actor 0.2 to 11 s after they came, let it go 0.7 to
  12 s after taking it, and took it again, over and over; a free one took it only 9.2 s after it came.
  Spiders the same, those within reach biting while they held it. The pack: hostile
  `nearest_attackable_target` `attack_interval` 5 s and `melee_box_attack` `random_stop_interval`
  100, none of it in `spider_angry`. Fixed: spiders and cave spiders look one tick in 100 and let go
  one tick in 100 unless their prey hit them (`searches`/`stops` in mob-traits.tsv; daylight.rs).
- Glass (fights.txt, corrected): BDS's zombie behind the glass wall took the actor only once it had
  strolled past the wall's end, as line 21 says; it does not see through glass. Sapling the same.
- A raider's `TargetEid`: see effects-raids.md (same).
- Sapling's run (sapling-targets2.txt, after the fix): the hard zombie took the actor 0.39 s after it
  came and hit every second from 1.4 s; the baby took it at 0.25 s but first hit at 4.4 s (BDS 2.4 s,
  not followed); the cave spiders took and let go over and over as BDS's, but one held 5 off bit
  (BDS's held ones never did; not followed).
- The baby's late hit (2026-11-05, `q3baby`, bds-q3baby.txt against sapling-q3baby.txt, three babies
  7 off on easy): BDS's first hit 43, 30 and 32 ticks after the summon, sapling's 60, 60 and 57. Both
  announce the baby with `minecraft:movement` 0.35 (the pack's `zombie_baby` group), but sapling hunted
  at the grown zombie's 0.23. Fixed: a baby hunts at its announced pace over the grown one's
  (`creatures/shown.rs` `pace_share`, `sapling_data::mob_pace`); test
  `a_baby_zombie_7_off_hits_within_45_ticks_of_its_summon`. BDS's babies took the actor 0.3 to 0.8 s
  later than sapling's (not followed).

## Ways (2026-11-04, `e1path`, research/captures/bds-e1path.txt)

A zombie 12 north of the survival actor by night, 15 s for each way (sapling-e1path.txt for sapling's):

- A trench two wide and three deep across the way: it walked in and stayed in it. Sapling the same.
- A shelf three up: it took the actor at 1.6 s, having strolled a quarter block on, dropped the three
  and hit at 6 s. Sapling's stood where it came and never saw the actor: its eyes' line to the
  actor's passed 0.007 into the shelf's edge, which the quarter block BDS's had walked cleared. A
  drop of three is taken (test `a_zombie_on_a_ledge_three_up_takes_its_prey_below_and_drops_to_it`).
- A one-deep pool three wide: BDS's crossed its floor at 0.45 blocks a second and climbed out; sapling's
  walked across at its land pace. Fixed: legs in a liquid push as in the air (`legs.rs`, 0.02 x 0.98 x
  its speed against water's drag of 0.8).
- A cobblestone wall with a wooden door: it never came through and strolled off; sapling the same.

## Reach, holes and walls (2026-11-05, `y3hunt`, research/captures/bds-y3hunt.txt)

By night on the stone floor, the survival actor held in place:

- A zombie that saw the actor, then had a stone wall two high and the floor's width put between
  them, kept its target the 14 s to the kill. Sapling keeps seen prey whatever comes between: same.
- A one-deep hole three wide across its way: it walked in and jumped out the far side, as sapling's.
- A zombie held 28 off: `TargetEid` set every 10 ticks and 0 the tick after, the pack's
  `within_radius` 25. Fixed: `within=` in `mob-traits.tsv` (zombie 25), `Hunts.within`; test
  `a_zombie_28_off_takes_its_prey_at_each_look_and_lets_it_go_a_tick_later`.
- A held zombie with the actor stepping round it turned its head 21 of 256 a tick (30 degrees, the
  Java game's look-at speed) and its body after, lagging 5 to 10 ticks; sapling turns both 60 a tick.
  Not followed (`legs.rs` ties head to body).
- A spider at midnight took no target in 4 s, so whether one keeps its prey by day was not seen.
- `y3gear` (bds-y3gear.txt): BDS sends `MobEquipment` for the skeleton, stray, bogged, wither
  skeleton, zombified piglin, piglin, brute, pillager, vindicator, drowned and vex as each is
  summoned; none for the zombie, husk or evoker.

## Not drilled

The kinds not named here, and what a blast leaves of the blocks it breaks and of softer ground.

## Sapling

Every kind that walks strolls (`creatures/strolling.rs`, `strolls=` in `mob-traits.tsv`, the
shard's `mobs_stroll`): a tick in 120 it picks the first of ten places up to 10 blocks off along
each axis that has floor, and walks straight there at its stroll speed, giving up at an edge, at a
block in its way or after ten seconds (how often: "Strolling" above, drilled). A mob a player hit, of a
kind with `panics=`, runs for 70 ticks from place to place within 5 blocks at that speed.

A creeper is lit, burns and bursts so (`creatures/blast.rs`): its blast hurts and throws players
by the rule above, hurts mobs, and breaks blocks along the Java game's rays, which break that one
stone. What a block withstands is BDS's own resistance (`block-resistance.tsv`, read off a live BDS,
and its drops as `Explosion::explode`'s, d719a4e7); the fuse goes out when its prey
is 6 off or gone, as the pack says: drilled 2026-11-05 (`q3creeper`, bds-q3creeper.txt against
sapling-q3creeper.txt), prey stepped 5.5 off a lit fuse let it burn on, 6.5 and 7.5 off put it out until
the creeper walked back, on both alike.

A skeleton stands within 10 blocks of its prey and looses an arrow every 61 ticks
(`creatures/arrows.rs`): aimed and spread as the Java game's, flying by the two numbers above,
sticking in blocks and hurting players whose box its own meets. It walks at its prey for its
first 19 ticks, which takes it BDS's 1.7 blocks; an arrow hurts a mob in its way too and cannot be
picked up. Run against sapling the drill's far skeleton still hits more often than BDS's, whose
arrows passed a hand over the player's head.

In the way (2026-11-04, `e1arrow`, bds-e1arrow.txt): a held skeleton 12 east of the actor, a held
villager halfway. BDS's seven arrows hit the villager three times (`HurtAnimation`, gone, `bow.hit`)
and never the actor; sapling's four (sapling-e1arrow.txt) all flew over the villager's head and hit
the actor. Not the in-the-way rule but the aim: BDS's arrows left at 1.5 to 1.7 across and -0.19 to
0.29 up, mean 0.01 up, a spread of about 0.14 a part, which is the pack's `uncertainty_base` 16 less
4 a difficulty level (12 on easy) x 0.0075 x 1.6; sapling's left at 0.18 to 0.21 up, its aim 0.6 up
the prey with 0.2 a block of loft and its spread 0.075.
Fixed 2026-11-04 (twelfth wave), from the decompile: `ProjectileComponent::shoot` (0x142401f40) takes
its aim from the shooter's pitch and yaw (0x142401750, a vehicle's when it rides), with no loft, and
its uncertainty as `uncertainty_base - difficulty x uncertainty_multiplier`, floored at 0; 0x1424020c0
makes the aim one long, adds a gaussian x 0.0075 x uncertainty to each part, then multiplies by the
power and adds the shooter's motion. The mob looks at its prey's eyes, so its arrow leaves level with
them (the far skeleton above, -0.15 to 0.28, fits that spread too). BDS's launch speeds ran 1.39 to
1.79 (bds-shoot.txt, bds-e1arrow.txt, bds-fights.txt), as the spread's lengthening gives.
`creatures/arrows.rs` `MOB_ARROW` (skeletons, strays, bogged, pillagers), tests
`a_skeleton_aims_level_at_its_preys_eyes_and_spreads_less_on_harder_levels` (mob_aim.rs) and
`an_arrow_hurts_a_mob_that_stands_in_its_way` (now a llama and a villager: a cow is lower than the
level arrow). Re-drilled (sapling-e1arrow.txt): seven arrows at 1.29 to 1.82 a tick, -0.25 to 0.30 up,
as BDS's; but they flew through the held villager 1.5 to 1.6 up its 1.9 and one hit the actor, where
BDS's hit the villager three times. The shard test's villager a block before the skeleton is hit, so
what lets the drill's farmer pass is left open.
Settled 2026-11-05 (twelfth7): the farmer was not there. `effect @e[type=villager] slowness` answered
`noTargetMatch` on sapling, as its villager is a `villager_v2`, and it strolled off; BDS's
`type=villager` takes `villager_v2` (as `type=zombie_villager` takes `zombie_villager_v2`). Fixed in
`command/selector/matching.rs` `is_kind`, test `the_older_villager_names_take_the_newer_villagers`.
Held then, sapling's farmer still strolled in its first second (a fixed dice draw, 5 ticks after the
summon) to (4.6, 1.4), off the arrows' line; the scene now puts it back once held (`eleventh1_drill.rs`).
Sapling's arrows also moved first by the launch speed less drag, where BDS's move by it
(9.902 - 1.5046 = 8.397, bds-e1arrow.txt): `creatures/flight.rs` `Unflown`. Re-drilled with the
farmer put back: BDS's six arrows (bds-e1arrow2.txt) all missed it, passing x 6.5 at z 0.13, -0.06, 1.05
or over its head, and missed the actor; sapling's (sapling-e1arrow.txt) hit it three times and the actor
once, as BDS's first run. Which is hit is the spread's draw on both.

A stray and a bogged shoot as the skeleton with their effect on the arrow, and a pillager while
it walks on, its crossbow heard loading where it stood at the shot before; it is not shown anew.

A wolf and a zombified piglin hunt none but a survival player that hit them, from that tick
(`hunting::provoke`), and so do an iron golem, with a hit of 7 to 21 and its throw, and a polar
bear, every 39 ticks without its rearing; a spider dazzled by the sun takes its hitter too. A
piglin hunts any survival player, gold worn or not; out of the Nether it turns into a zombified
one 303 ticks on (mob-kinds.md, `t2barter`). A
piglin's fellows did not join in on BDS either (bds-gxfellows.txt: of a piglin 9.5 off in the open
and one shut in stone 12 off, neither took a target when the third was hit, the pack's
`broadcast_anger` notwithstanding), and a golem guards no villager. A hit piglin refuses gold for
the pack's `cooldown_after_being_attacked` 20 s (bds-gxbarter.txt: an ingot 2 s after the hit was
refused; `piglins::Wary`), and one admiring gold is heard `admire` again the tick its hand
empties while a player within 8 holds gold (none when nobody does, bds-goldpickup.txt). A llama
hit spits once, 41 ticks after, for 1 and no throw (`arrows::SPIT`, aimed as the Java game's), and
lets go 79 ticks after; 21 ticks before the spit it is heard `warn` (0.64 of the way toward its
prey) with `CHARGED` and `ChargeAmount` 1, cleared 2 ticks before the spit (bds-provoke2.txt;
`shooting::charge`). A panda and a bee hit take their hitter, hit it once and let go (`once`); a
panda's other genes: `mob-kinds.md`, "Pandas' genes". A panda stands 48 ticks after taking its
hitter before it walks up (bds-provoke3-findings.md; `genes::stands`).
A bee flies at its hitter (`fights=flies`, creatures/bees.rs), angers the bees within 20, poisons on
normal and hard, and dies 10 to 60 s after its sting (2026-10-27).

A hoglin, a zoglin and a warden hunt as above with their numbers (`varies=` draws the hoglin's and
zoglin's hits). The warden (`creatures/warden.rs`, research/captures/bds-warden-findings.md) sniffs,
looks at what disturbs it, grows angry at the player behind a step or a touch, roars, then hunts;
it booms through armour at prey it cannot walk to, and climbs out and digs down (2026-10-28).
A creaking (`creatures/creaking.rs`) freezes under a look within 60 degrees of its middle from 16
blocks, the Java game's rule; its heart: `creaking/heart.rs`. Drilled (bds-gxcreak.txt): taking
prey, `TargetEid` at T, then at T+1 `mob.warning`, `activate`, `CALM|OBSERVED` and state 1; prey
lost (the actor turned creative), `TargetEid` 0 at T, then at T+1 `deactivate` from its feet, `CALM`
only if it was frozen, its ambient sound `ambient` and state 0, staying frozen; looked away from
while hunting, it unfreezes with `CALM` and state 2. Sapling does the same (test
`a_creaking_whose_prey_turns_creative_deactivates`).

`creatures/hunting.rs`, for kinds with `attack=`, `speed=` and `sees=` in `mob-traits.tsv`: the
zombie and the seven kinds above, each with its numbers and what its hit gives (not the cave
spider's poison, which lasts no time on easy).
The first two hits of `bite` and both of `slain` land where BDS's did, to the last digit.
Its jump over the step of `chase` is BDS's to the sixth decimal.
Where the floor between is not free it goes by the shortest way over whole cells
(`creatures/paths.rs`), looked for again every 10 ticks; run against sapling the two fences were
passed by the same cells and the first hits came 4.1 s and 6.5 s after. Strolling and fleeing
mobs do not look for a way.

Different: it sends a mob's speed as it is before the drag, turns 60
degrees a tick with its head (BDS 30, its body after: "Reach, holes and walls"), stops when its box touches the
player's since players and mobs do not push each other, shows no mob holding
its weapon, lets a spider hunt by day, hunts nobody on peaceful
(where BDS has no monsters), and tells every viewer every move.
