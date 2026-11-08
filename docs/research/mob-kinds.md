# Mobs that fight their own way

Kinds that throw, cast, teleport, ram, leap or fly, each drilled on BDS like those in
`mob-attacks.md`, which covers picking a player, walking, hitting and the plainer shooters.

## The witch (`witch`)

A witch 6 and then 12 blocks from the actor in survival at midnight, the actor healed every second
(`research/captures/bds-witch.txt`, `bds-witch-findings.md`):

- The near one took the actor as it came, walked at it 19 ticks at 0.1349 a tick (movement 0.25)
  and stood 4.27 off; the far one took no target. It let go once the actor was 10.5 off.
- It threw a `splash_potion` every 60 ticks, the first 61 after its target: 1.415 above its feet
  and 1.15 along the aim, at 0.75 a tick with no spread, slowed 0.99 and falling 0.05 a tick. Its
  rise grew with the distance, 3.35, 11.48 and 19.25 degrees at 4.27, 6.19 and 8.16 blocks, which
  is not the Java game's formula.
- The potions followed its pack: poison first (health 8 or more, not poisoned), then harming,
  slowness once the actor was 8 off and not slowed. Every one hit the actor and gave it the full
  drinkable effect: poison for 900 ticks, slowness for 1800, harming 6 and a throw 0.4 away and
  0.4 up. Each break sent a `ParticleSplash` in the potion's colour and a `glass` sound.
- Its own poison, splashed 4 blocks off, lasted it some 97 ticks; that is all the capture says of
  one not hit.

Drinking (`witch-drinks`, `bds-witch-drinks.txt`; the actor in creative, so the witch had no prey):

- A witch hurt by 6 drank healing (data 21) 12 seconds on; one stood in fire drank fire resistance
  (12) 3 seconds on and healing twice after the fire hurt it; one put under water floated up and
  drank nothing. Each held the potion (`MobEquipment`, `potion` of its data) 33 ticks, then an
  empty hand, and had its effect. Its pack's `drink_potion`: water breathing under water 0.15,
  fire resistance on fire 0.15, healing when hurt 0.05, speed with prey 11 or more off 0.25.

## The enderman (`ender`)

Three runs on the flat ground, the actor in survival at midnight and healed each second
(`research/captures/bds-ender-findings.md`; run 2 `bds-ender-wet.txt`, in water; run 3
`bds-ender.txt`, a dry stone floor):

- Every enderman of five teleported on its first tick. Each teleport is a `MoveEntityDelta`
  marked `force_move`, of the axes that changed, with a `SoundEndermanTeleport` LevelEvent where
  it left and where it came; it landed within 16 along each axis, standing on the ground below.
- Calm, it strolled at 0.1943 a tick (movement 0.3). One calm enderman teleported some 5 seconds
  apart, another not once in 520 ticks.
- Stared at (`stare`, `bds-stare-findings.md`, the bot's look held by `!look`): a look 4.87
  degrees off its head (2.465 up it) at 6 off for 5 ticks took the actor, then `ANGRY`,
  `mob.warning` and `stare`, and a teleport 2 ticks later; one 9.42 off did not. The Java game's
  `1 - 0.025 / d` (5.2 degrees there) fits; whether the enderman faced the actor did not matter.
- Seven stares from 6 off, the look set by `/tp ... facing`, took no target; nothing shows the bot
  held the look.
- Hit, it took its hitter that tick, was thrown 0.4 and 0.4 up, teleported 0 to 2 ticks after,
  and showed `ANGRY` with `mob.warning` the tick after. It walked at 0.4371 a tick (movement
  0.45) and hit for 4.5 on easy (its 7) every 20 ticks, throwing the actor 0.4 and 0.4 up.
- Left more than 16 off, it teleported 30 ticks later to some 16 nearer, give or take 4: the
  Java game's `teleportTowards`. Twice it teleported away from beside the actor for no cause seen.
- It never let its target go: 32 seconds, out to 24 blocks. Water hurt it 1 and sent it off.
- Rates (`q1ender`, `q1ender2`, fourteenth1q; count with `SoundEndermanTeleport` pairs): four calm endermen
  made 77 teleports in 14400 enderman-ticks at midnight (0.0053 a tick) and 132 in 6400 at noon under the sky
  (0.021), both about 0.4 of the attempts `TeleportSystem`'s chance and timer give. The random teleport
  (0x142443ff0) draws one point in the cube and tries only it (0x142443220), so many attempts fail; sapling
  tries one spot for a chance or timer teleport since fourteenth1q (`teleports.rs`). The roofed phase was spoilt
  by endermen leaving the roof's cover.

## The goat (`goat`)

Three runs at noon on easy (`research/captures/bds-goat-findings.md`; `bds-goat-wet.txt` and
`bds-goat-one.txt` one goat each, `bds-goat.txt` eight penned with the actor):

- A goat took the nearest player some 8 ticks after it came, with `mob.warning`, and dropped it
  over 16 off; holding it, it only stood and strolled (0.1243 a tick, movement 0.4 by 0.6).
- None rammed in its first 77 seconds, one goat not in five minutes; the first rams came at 77,
  257 and 292 seconds: its `cooldown_range` 30 to 300 is in seconds, from its coming and from each
  ram.
- A ram: it ran at 0.1690 a tick to a place due north, south, east or west of its prey, 7 off
  where the floor let it (5 to 6 by the glass), stood, sounded `pre_ram` a tick later and charged
  22 ticks after that along the axis, its steps 0.0282, 0.0583, 0.1131, 0.1888, 0.2859, 0.4043,
  0.5458, 0.7090, 0.8950, 0.9965, 1.0521.
- What it met took 2 on easy (`HurtAnimation` 30, a baby's 1) and two throws that tick: a hit's
  0.4 away and 0.4 up, then half that and a push of 1.5 s² along the charge and 0.18 s² up, s
  its step then. `attack` and `ram_impact` came the tick after. A player it could not hurt took
  the push alone.
- Each ram met someone, so a miss was not seen. The goats leapt the glass, 3 high.
- With cows and the actor in creative (`goat-cows`, `bds-goat-cows-findings.md`), every goat took
  a cow, never the actor, the watcher or a goat. A cow rammed lost 1 to a baby and was thrown in
  one motion, half a hit's throw and the push, then ran. A cow that walked off left the charge to
  miss, with nothing sent; the watcher in the way took the push.

## The evoker (`evoker`)

An evoker 10 off for thirty seconds, then one held still 2 off for fifteen, the actor in survival
at midnight on easy and healed each second (`research/captures/bds-evoker-findings.md`):

- It took the actor within 20 and began a cast that tick: `EVOKER_SPELL` with the spell's colour
  (fangs `0xFF664D59`, vexes `0xFFB3B3CC`), `cast.spell` a tick later, standing still for the 39
  ticks the flag showed. The next cast began 101 ticks after a fang cast ended, 342 after a vex one.
- Prey 3 off or more got a line: 16 `evocation_fang`s a tick apart from 21 ticks after the cast
  began, fang i at the floor of the evoker's x and z plus 1.25 (i + 1) towards the prey, y the top
  of each column's ground. Nearer prey got two rings, with `prepare.attack` at the start and again
  with the first: 5 fangs 1.5 out and, 3 ticks later, 8 at 2.5, both from the angle to the prey.
- A fang came with `LimitedLife` 22, swung (`ArmSwing`) and bit the tick after, and sounded `throw`
  and `fang` the tick after that; it went 11 ticks after it came. A bite cost 6 on easy by magic
  (`HurtAnimation` 14) and threw 0.4 away from the evoker and 0.4 up; one bite a cast landed.
- An evoker that lost its prey ended its cast at once, after 5 of its 16 fangs.
- Its vexes (one cast in three at 10 off) came three, a block from it, flew through blocks,
  charged at 0.91 v + 0.05 a tick and hit for 5.5. It stepped away from a player 2 off, as its
  `avoid_mob_type` says, and walked between casts for a reason not seen.

- Its vexes (`bds-evoker-findings.md`, "Vexes"): three at whole-block offsets (+1, 0), (-1, -1),
  (-1, +1) from it at its feet' height, each with an iron sword and 14 health, no collision or
  gravity; they took the actor 2 ticks on and re-sent it every tick with `mob.warning`. A charge
  set `CHARGE_ATTACK` with `charge` a tick later and went `v = 0.91 v + 0.05` at the actor's eyes,
  ending at a hit (5.5 on easy, a throw 0.4 along its flight and 0.4 up, `attack` a tick later);
  between charges they wandered at some 0.135 a tick. The summoning showed 99 ticks, the vexes
  came 2 later with `prepare.summon` again, and the next cast began 342 after.

## The dolphin (`dolphin`)

A dolphin in a pool 13 across and 7 deep at midnight on easy, the actor at its bottom breathing
water, held still by slowness for one hit and healed each second
(`research/captures/bds-dolphin-findings.md`; three earlier runs lost the pool or the hit):

- In water it moved by `v = 0.8 v`, then a push of 0.022 along its heading, no gravity: 0.11 a
  tick at most, calm or chasing; slowness did not slow it. It turned 10 degrees a tick at most.
  Calm, it sank to the floor and swam about it; it never came up for air in the minute.
- Its `Air` counted down from 4800, a tick at a time, filled again at a breach.
- Hit (9 of 10 left), it took the actor the next tick, then `ANGRY` and `mob.warning`; it chased
  at the same 0.11 and bit every 20 ticks within 0.8 across of its box: 2.5 on easy, a throw 0.4
  away and 0.4 up in water. It breached once mid-chase, out of the water 7 ticks.
- It was still angry 32 seconds on, past its pack's 25.

`f14edolphin` and `f14edolphin2` (research/captures/bds-f14edolphin.txt, trimmed to the actor's
stream; bds-f14edolphin2.txt), three dolphins in an open pool, one on a dry stone tray, one under
a stone cap:

- Anger: hurt by `/damage` as from the actor, the dolphin took it in the hurt's tick (its health
  told with it), as sapling's did; the two others (5 and 8 off) took it the tick after, each
  `mob.warning` and `ANGRY` a tick after its `TargetEid` (sapling's took it with the hit one until
  fourteenth2h, `anger::join`). All
  three let go when the actor went creative; each `ANGRY` cleared 24.9 s after that dolphin's last
  bite (one bite at most 10 s before, the hit one's 4 s before): 25 s from its last bite.
- On land: a tick after coming to rest it hopped, its first tick's rise 0.4216 (0.5 on a
  grounded tick's 0.0784 fall), up to some 0.2 across, a random yaw, then each tick's fall 0.98 of
  the last less 0.1784; heard `flop` a tick after each hop. After 2400 ticks out of water (2401
  after it landed, 2407 after its summon in the air) it was hurt 1 every 10 ticks with no cause
  (`HurtAnimation` -1) until it died, 10 hits on.
- Under the cap its `Air` ran down 4800 a tick at a time; at 0 it drowned, 2 every 10 ticks
  (`HurtAnimation` 9). In the open pool none came up in 4 minutes but on running low: at `Air` 140
  to 148 each swam up at up to 0.2 a tick, breached (`SPRINTING`), and filled its `Air` again.

## Pandas' genes (`pandas`)

A panda of each of five genes, summoned with its gene's event, held still and hit once, the actor
in survival at midnight on easy (`research/captures/bds-pandas-findings.md`; `provoke3` holds the
normal, lazy and playful ones of an earlier run):

- `Variant` showed the gene: lazy 1, worried 2, playful 3, brown 4, weak 5, aggressive 6; a weak
  one had 10 health. BDS rolled two summoned ones' genes anew a moment after (6 to 3, 2 to 4).
- Hit, each took the actor as its target; only the aggressive one bit, 8 times in 20 seconds,
  4 on easy (its 6), a throw 0.4 and 0.4 up, walking at it at 0.0486 a tick, and never calmed.
  The lazy (0.066 a tick), worried, weak and playful (some 0.075) ran instead, 24 to 95 ticks.
- A worried one kept from the actor before it was hit, sprinting while nearer than 7.
- Every gait fit `2.16 (movement x multiplier)^2` a tick.

## Effects on mobs (`mob-effects`, `mob-effects2`)

`/effect` on cows and zombies at midnight, the actor in creative
(`research/captures/bds-mob-effects-findings.md`):

- A player's rules held: regeneration 1 a 50 ticks, poison 1 a 25 down to 1 and no lower, wither 1 a
  40, instant damage 6 and health 4 on a cow, resistance 20% a level, speed's 20% a level on the
  stroll to 1e-5, slowness 255 standing a zombie still; fire resistance stopped a zombie's burns in
  netherrack's fire, water breathing a cow's drowning under glass. A timed effect turned on the tick
  it came, then whenever its time left was a whole number of turns, not at 0.
- An undead one: instant damage mended it 6, instant health hurt it 4 unflashed; poison and
  regeneration were not given it at all.
- Every effect showed in `VisibleMobEffects` (each id doubled, 7 bits apiece, the lowest highest);
  `MobEffect` came for speed and slowness alone. A cow an effect hurt ran.

## The four newest effects (`effects3`, `wind-burst`)

A zombie held still and given each, then killed, the actor in creative
(`research/captures/bds-effects3-findings.md`; potions 43 to 46 gave ids 32 to 35, three minutes,
in `bds-drink2.txt`):

- Named `effect.wind_charged`, `effect.weaving`, `effect.oozing` and `effect.infested`.
- Wind charged: dying, it burst at its middle (`ParticleWindExplosion`, `wind_charge.burst` a tick
  later) and pushed both players towards their eyes; by the distance from its feet, about
  `0.82 (1 - d / 9.8)` (`bds-wind-burst.txt`, seven samples from 1 to 7 off, noisy).
- Weaving: 2 cobwebs, its own cell and one beside, each with `AnimationSpawnCobweb`.
- Oozing: 2 slimes of the second size (4 health) at its feet, half a block up; killed, each split
  into tiny ones.
- Infested: 2 of 10 hurts let out a silverfish from its middle, at 0.3 near its facing.

On a survival player killed with each, the watcher 3 off (`player-omens`,
`research/captures/bds-player-omens-findings.md`), the same, from where BDS puts a player, its eyes,
and a dead one's box 0.2 high: the burst at eyes + 0.1 with data 3 (a zombie's was 4), the watcher
pushed 0.542 (the rule from the eyes gives 0.535), the dead player not; 2 cobwebs about the eye
cell's floor; the slimes at eyes + 0.5. Infested let out silverfish on 1 hurt of 10, two at once
(the Java game's 1 or 2), at eyes + 0.9, at 0.3 within 90 degrees of its facing. Its effects were
not ended at death but at its rise, their time frozen.

## The shulker (`shulker`)

A shulker 6 off the actor in survival at midnight on easy, healed each second
(`research/captures/bds-shulker-findings.md`; the 12-off half was lost to a fall death):

- It took the actor in the same packet as `ShulkerPeekId` 100 and stayed open while it held it,
  `shulker.open` a tick later; it let go some 15 off, closing (0, `shulker.close`). It never moved.
- It shot 37 ticks after taking its target, then 36, 41 and 45 apart: a `shulker_bullet` from its
  feet + 0.692 up and 1 towards the target, still, with `shoot` and `undefined` 0.99 up it.
- A bullet flew as the Java game's `ShulkerBullet`, to 1e-4: a pull of 0.15 along one axis, grown
  ×1.025 a tick, its speed eased a fifth of the way to it each tick, no gravity or drag. It chose
  another axis towards its target where it came level with the target's middle cell, or after 10,
  20, 30, 40 or 50 ticks; its first leg could go up. 25 ticks took it to the actor 6 off.
- It met the actor's box 0.3 wider: 4 on easy (`HurtAnimation` 3), a throw 0.4 away and 0.4 up even
  in mid-air, and levitation 200 ticks (refreshed by each next hit); it went the tick after with
  `bullet.hit` where it met.

## The breeze (`breeze`)

A breeze 8 off the actor in survival on stone at midnight on easy, fall damage off, healed each
second (`research/captures/bds-breeze-findings.md`; leftover blocks stood about, as the air fill
was over `fill`'s limit):

- It took the actor 6 ticks after it came. Before each shot `FlagsExtended` bit 24 showed for 16
  ticks; a `breeze_wind_charge_projectile` left its feet + 1.416 at 0.7 a tick, aimed at the
  actor's middle, with no gravity or drag. Bit 24 went 4 ticks after the shot.
- Then, 4 to 20 off, it readied a leap (bit 39) for 10 ticks and leapt at 40, 55, 60, 75 or 80
  degrees, at most 1.4 a tick: no drag across, 0.08 a tick more fall, and 0.2655 higher on its
  first tick. It landed 1 to 4 off the actor, by a rule not found. Nearer, it walked out to 4 to 8
  off at 0.497 a tick, or stood 10 ticks.
- A charge that met a player stopped there and burst 6 ticks later: 1 on easy (`HurtAnimation`
  3), a hit's throw 0.4 from the breeze and 0.4 up, then the burst's push on top, `0.6 (1 - d/6)`
  towards the eyes, d from the feet; every other player within 6 got the push alone. One that met
  a block burst there. `breeze_wind_charge.burst` came the tick after, as it went.

## The ghast (`ghast`)

A ghast 16 off and 8 up, then one 12 off and 6 up held still by slowness, over the actor in
survival on obsidian at midnight on easy (`research/captures/bds-ghast-findings.md`):

- It floated with no gravity, `v = 0.91 v + 0.03` towards a whole-block spot up to 10 off across
  and 7 up or down, and picked another within a block of it; it wandered out of its 28 range.
- 20 ticks after it took the actor it opened its mouth (`CHARGED`, `ChargeAmount` 1) with `warn`,
  closed it 19 later, and shot 21 after the warning; it warned again 60 after each shot, 81 ticks
  a round, without fail.
- A fireball left its feet + 0.5 up and 1 towards the actor, aimed at the actor's middle at 1.6 a
  tick, no gravity or drag; the ghast's box pushed it aside its first ticks. One that met the
  actor stood 6 ticks, then hurt it 6 (`HurtAnimation` 3) and set it alight; the blast of power
  1 came the tick after, `7 (a² + a)` with `a = 1 - d/2`, d from the feet, the same on every
  difficulty, and lit fire about it. One that met the floor blew up there.
- A fireball through a standing player (`q1ghast`, bds-q1ghast.txt, fourteenth1q; a stone wall behind the
  actor): it froze 6 ticks and moved once more, clipped by the wall. The `SetEntityMotion` changes after
  the freeze lay along its own travel line (one shot +0.553, another -0.036), not from the player to it,
  so a player's push was not seen; the wall spoils the run.
- Without the wall (`tt1ghast`, bds-tt1ghast.txt, fourteenth1t; a warded survival actor, open ground
  behind, 12 shots from four spots): a fireball that meets the player ends its whole step (its box
  into the player's), stands still 5 ticks while its motion takes the player's push (Java's
  `pushAwayFrom` at 0.1, the player's box grown 0.2, once a tick: 5 or 6 pushes fit, cos 1.000; none
  under 0.01 apart), and 6 ticks after the meeting hurts the player and makes one more step of that
  motion, cut short by blocks; the blast comes the tick after. Sapling: fireballs.rs, the same.

## The snow golem (`snowgolem`)

A snow golem on plains at noon with a zombie 8 off (`research/captures/bds-snowgolem.txt`):

- A layer of snow came the tick after it was summoned under its feet, and as it walked one under
  each of the four cells a quarter block about where it stood at the tick's start, in one update
  where several were new (temperature under 0.81, the pack's `trail`).
- It took the zombie (`TargetEid` the zombie's unique id, `mob.warning` a tick later) once it was
  within 6, walked at it, and threw its snowball 22 ticks after taking it (Java's `RangedAttackGoal`
  throws at 21 with a 20-tick interval), from about its eyes at 1.5 a tick, heard `shoot` from its
  middle. The zombie, which had taken the golem 4 ticks after it came, killed it (health 4) first.
- Heat melts it by the pack's `hurt_on_condition` cause `temperature` (wire 26), not fire; the API's
  `DamageCause::Temperature` is that (2026-11-04; plains is too cool to melt it, so not drilled).
- Water hurts it, and a blaze, as an enderman (`o3wet`, research/captures/bds-o3wet.txt): in a pool
  and in the rain, 1 each 10 ticks with `HurtAnimation` 9; the golem's 4 health went in four.
  Sapling: `creatures/harm.rs` `WATER_HURTS`, test
  `water_and_rain_hurt_a_blaze_and_a_snow_golem_1_each_10_ticks_as_drowning`.

## Drowned with tridents (`drowned`)

A drowned summoned `as_ranged_adult` in a pool 8 from the actor at midnight
(`research/captures/bds-drowned.txt`): shown holding a trident (`MobEquipment`) as it came; it took
the actor and 77 ticks on swung (sapling waits Java's 20 to 60 by distance), a `thrown_trident` following a tick later at 3.94 a tick (the
pack's power 4), heard `item.trident.throw` from the drowned. It kept its own trident; the thrown
one stuck in the floor. One handed a trident by `replaceitem` (an earlier run) only bit.
The throw wait (`s3drowned`, 2026-11-05, bds-s3drowned.txt, three drowned 6 to 10 off in a block of
still water 6 deep, and bds-q3drowned.txt): BDS's swings came 48 to 84 ticks apart (60, 59, 84, 60,
79, 60, 67 here; 48 to 72 in q3), with no link to the distance (2.1 to 11.8 at the throw), and the
first 72 to 93 ticks after the drowned took its prey (outliers 134 and 175). The pack's
`attack_interval` 1 to 3 s scaled by distance (Java's) gives 20 to 60. `RangedAttackGoal::tick`
(149fe7ec0) lerps two fields by distance over the range and waits on sight and a facing check
(141c640b0) before it shoots; the field labels are garbled, so the fit stands: a wait of 48 to 84
after each throw, and 20 more before the first (`creatures/drowned.rs`, test
`a_drowned_throws_48_to_84_ticks_apart`). BDS's drowned threw from 2.1 off too.

`drownedloot` (`bds-drownedloot.txt`): 40 drowned summoned `as_ranged_adult` in a pool and killed by
`damage ... entity_attack entity @s`. All 40 held a trident; 5 held a nautilus shell in the offhand
(`MobEquipment` window `Offhand`, slot 0; the table's 8%). 13 tridents dropped (sapling: a quarter, of
mobs a player killed, Java's equipment rule), each with a `Damage` tag of 32 to 248 (sapling: Java's
`250 - nextInt(1 + nextInt(247))`), none enchanted; all 5 shells dropped (the pack's offhand chance 1.0).
Besides: rotten flesh, copper ingots. sapling: `creatures/drowned.rs`.

## Paintings (`paintings`)

BDS's pictures are `Motive`s built in one static initialiser (`142a1ce60`, each at `1429f4250` with
its width and height in pixels and a flag for whether a hung painting may take it: Earth, Wind,
Fire and Water may not). The painting's constructor (`1429fbbe0`) walks them in order, keeps each
that survives (`1429cce40`: no collision in its box, a solid block behind each cell, no hanging
actor in the box) at an area no less than the best so far, drops those smaller than the last best,
and takes one by the level random's `nextInt`. In `research/captures/bds-paintings.txt` west faces of
a wall at x 3 gave a 4 by 4 (`unpacked`, `BurningSkull`) on an open wall, `Sea` in a 2 by 1 gap and
`Bust` in a 2 by 2 one, each `AddPainting` at the wall less 1/32 and the middle of its cells, moved
half a block to the viewer's left and up for an even width and height (Java's offsets). sapling
(`paintings.rs`) does the same with "solid" as collision, and keeps the painting with its column.
A hit or a lost wall breaks one, dropping a painting outside creative (ba5d1a76, `paintings/falls.rs`).

## Armour stands (`armorstand`)

`ArmorStand::getInteraction` (`1429b5e20`): nothing for a spectator; a sneaking click is
`action.interact.armorstand.pose`, which moves `PoseIndex` on by one of 13 (`142a044f0`). Otherwise
the height of the point hit above the stand's feet picks a slot, in this order: 0.1 to 0.55 feet,
0.9 to 1.6 chest, 0.4 to 1.2 legs, 1.6 up head, below 0.1 none. A held armour piece goes to its own
slot, anything else to the main hand; an empty hand takes from the slot clicked, the main hand first
when the chest is clicked. The swap (`1429ba2c0`) does nothing for the same stack on both sides; a
creative player's item is copied onto an empty slot; from a stack of two or more one goes on and the
stand's item goes to the inventory (refused without room); otherwise the two trade places. Then
`mob.armor_stand.place` at the stand's position plus 0.5 on each axis. In
`research/captures/bds-armorstand.txt` a creative player's diamond helmet went on (main hand, offhand
then armour `MobEquipment`s, `armor.equip_diamond` from the stand's eyes, then the place sound) and
the player kept it; a second click with it did nothing. sapling (`creatures/armour_stands.rs`)
keeps the gear and pose with the stand (`Armor`, `Mainhand`, `Offhand`, `Pose`/`PoseIndex`, the
names the stand's save at `1429b4d80` writes). Which slot heads, skulls and pumpkins take, and
where the stand's old item goes on a plain swap (here: the player's hand) are [U].

Burning and hits (`o3stand`, `o3stand2`, research/captures/bds-o3stand{,2}.txt): lava scorched as fire
did (0.05 each 10 ticks). Out of it, the burn ran 160 ticks: a flinch and 1 off at each 20th but the
last, then alight no more; burnt up, the stand went unflinched and dropped its helmet and sword, not
itself. Its armour warded each loss by the Java rule (iron helmet: 0.05 → 0.04605, 1 → 0.94;
diamond: 0.010025 and 0.21). A second survival hit 2 or 4 ticks after the first broke it, 6 to 20
did not (Java's 5). Sapling: `armour_stands/burning.rs`, `hits.rs`; test
`a_dressed_stand_burns_through_its_armour_drops_its_gear_if_burnt_up_and_goes_out_after_eight_seconds`.

## Piglins bartering (`barter`)

A piglin in the warded room (`research/captures/bds-barter.txt`) picked a thrown gold ingot up: shown
in its offhand (`MobEquipment` window `Offhand`, slot 1) that tick, `admire` from its head a tick
later, and 161 ticks on (`admire_item` 8 s) the ingot went to its main hand and the offhand emptied.
No barter came out in three runs: the clicks with an ingot fell out of reach. sapling barters on the
click (the pack's `minecraft:interact` with `barter`), with that timing, and drops a roll of
`piglin_barter.json` (read by `mob-loot.py` with the block tables) as the offhand empties.
Admiring shows as `FlagsExtended` bit 30 (33685504 at rest, bits 17 and 25), sent right after the offhand
each time it fills and empties (bds-goldpickup.txt; sapling since fourteenth2h, `piglins::shown_admiring`).
Other shareables (bds-o1piglin.txt): a golden helmet is admired like a sword; cooked pork is taken and eaten
unseen; a crossbow goes to an empty main hand, shown a tick on. A `/kill` dropped whatever it took up (the
helmet and sword still or once admired, the crossbow); sapling since fourteenth1o (`piglins/pickup.rs`).
Weapons and armour by the pack's `shareables` priorities (bds-q1piglin.txt, `q1piglin`; sapling since
fourteenth1q, `piglins/gear.rs`): it took a crossbow over an iron sword, an iron sword into an empty hand, a
diamond helmet over an iron one and leather boots onto bare feet, and refused an iron sword over a crossbow, a
crossbow over a golden sword and an iron helmet over a diamond one. What it took was shown a tick on
(`MobEquipment` slot 0, or for armour the equipment slot, 2 head to 5 feet, `selected_slot` 120, window
`Inventory`); what it put down dropped a tick after that, 0.125 over its feet. At death what it took up drops;
`/replaceitem` armour does not. Unexplained: it left an iron helmet by its bare head.

`piglinkeep` then `piglinkeep2` with `KEEP_WORLD=1` (`bds-piglinkeep.txt`, `bds-piglinkeep2.txt`): a
piglin picked up a thrown golden sword, admired it in the offhand, and 161 ticks on was sent the sword
in its main hand, then the offhand emptied, the same tick; it then took a thrown ingot into the
offhand. After the restart it was announced with the sword (`Damage` 0 in its tag, as thrown) in the
main hand and the ingot in the offhand (slot 0, as a stored mob's hands are shown) and bartered
nothing in the next 11 s. sapling (`creatures/piglins.rs`) keeps both hands with the piglin
(`Mainhand`, `Offhand`; it also writes the armour stand's `Pose`, which BDS's piglins may not have) and
does not resume admiring after a load.

`t2barter` (2026-11-04, research/captures/bds-t2barter.txt, sapling-t2barter.txt): a piglin handed an
ingot by a click emptied its offhand 161 ticks on and threw its roll the tick after, from its feet plus
1.125, at 0.3 up and 0.2 along the way to the nearest player's eyes within 10 (BDS 0x14190f060; none
near: ±0.2 at random). Sapling threw it as the hand emptied, from where it stood; fixed
(`piglins.rs` `hand_back`, test in tests/integration/piglins.rs), the same in the re-drill. BDS's piglins
in the overworld shook (`VIBRATING`) a tick after they came and became zombified piglins 303 ticks after,
hands kept: `AddEntity` of the `zombie_pigman`, its `MobEquipment`, the piglin's `RemoveEntity`, then
`converted_to_zombified` at its middle, no `spawn` (the pack's `zombification_sensor`, `start_zombification`
15 s, `become_zombie`). Ported 2026-11-04 (`creatures/zombifying.rs` on the shared `creatures/turning.rs`;
test `a_piglin_out_of_the_nether_shakes_and_turns_zombified_303_ticks_on_keeping_its_sword`).

## Witches and endermen again (2026-10-29, `bds_command_probe kinds3`, research/captures/bds-kinds3.txt)

Sapling's run: sapling-kinds3.txt.

- A witch threw every 3.05 s or so (sapling 3.0). Its poison splash gave the actor it hit 900 ticks
  on both; a watcher beside the actor got 540 on BDS, 379 on sapling. Drilled 2026-11-03
  (`bds_command_probe splashshare`, research/captures/bds-splashshare.txt): a dispensed poison
  potion broken on the actor at (3.15, -25.80, 4.5) gave watchers 2.1 south, 2.1 east and 1 north
  506, 428 and 754 of 900: 1 - d/4, rounded, for d the gap to the box grown by 0.05 a side (the
  plain box gap gives 495, 416, 742). Fixed (`creatures/splashes.rs` `share_of`, its unit test). The
  potion's flight differs (sapling's broke at y -26.07, BDS's -25.80), so the run's shares still
  differ by a few ticks. A splashed instant damage sent no `MobEffect` on BDS; sapling sends an add of 1 tick.
- Splash instants by share (research/captures/bds-t2esplash.txt; `thrown_potion_effect` vtable
  0x14240ede0): the struck actor gets share 1, others `1 - d/4` as above. An instant's amount is
  `(int)(share × base × 2^amplifier + 0.5)` (harm 3, 3, 5 and heal 2, 2, 3 for watchers 6.6, 5.6
  and 3.5 off), and no MobEffect is sent; a lasting effect is given only if
  `(int)(share × ticks + 0.5)` is over 20. Fixed (`vitals/splashed.rs`, test
  `bds_watchers_were_harmed_and_healed_by_their_share`).
- An enderman stared at from 6 off, with and without a carved pumpkin, took no target in either
  run, so the pumpkin was not judged. BDS flipped its `MOVING` flag throughout. `q3stare`
  (2026-11-05, bds-q3stare.txt against sapling-q3stare.txt, foes_drill.rs's look at the head 6 off):
  twice in a pumpkin no `stare`, twice bare one `stare` on both sides. The pack's `looked_at` filter
  skips a player wearing one; sapling's took it before; fixed (`creatures/stares.rs`, test
  `an_enderman_takes_no_starer_in_a_carved_pumpkin`).
  The stare's angle (`s3stare`, `s3stare2`, 2026-11-05; read with the trials where the enderman
  stayed on its spot: one that teleports off, as most do within a few seconds, is no trial): at
  6.06 off BDS's stared at 0.3, 1.6, 2.2, 2.8 and 3.4 degrees and not at 6.0 or 9.0; at 12.03 off at
  0.2 and 1.8 and not at 9.7. The Java game's `1 - 0.025 / d` (5.2 degrees at 6, 3.7 at 12) and the
  earlier 4.87 at 6 agree with all of it; sapling's stared at the same trials.
- Endermen in the rain were hurt 1 every 10 ticks, cause 9, and teleported often. Sapling's took
  nothing; now rain hurts them as water does (`creatures/harm.rs` `WATER_HURTS`, tested in
  tests/integration/mob_heat.rs), without the teleports. The drill_server rerun still showed no
  rain hurt (see mob-damage.md, "Fire, magma, rain").
- The evoker and its vexes, re-drilled 2026-11-04 (sapling-evoker.txt, sapling-vex.txt against
  bds-evoker.txt, bds-vex.txt): an evoker's vex now tells its prey again every tick with `mob.warning`
  (test `an_evokers_vex_tells_its_prey_every_tick`), and is not heard to `spawn` (BDS's three were
  not; `vexes.rs` `summon`). Left: sapling's evoker summoned on its first cast where BDS's cast its
  line (sapling draws a summoning one cast in four; one BDS cast is too few to judge), let its prey go
  18 s in where BDS's held it, and so cast no fangs before the second came; a vex spawned by command
  still charges here. Checked 2026-11-04: the drop is distance, not time. Sapling's evoker strolled
  south after its summoning until the player (6.5, 20.5) was past its pack's `max_dist` 20, which sapling
  keeps prey within; BDS's strolled east and stayed within about 15. Whether BDS keeps a target out to
  `follow_range` (64) was not seen, so the keep range was left as it is (sapling-evoker.txt,
  bds-evoker.txt, `TargetEid` and `MoveEntityDelta` over time).
- Commanded vexes, traced 2026-11-07 (thirteenth3am, bds-g11vex-2.txt, four summoned vexes):
  open, not fixed. Two (20, 22) stayed above the stone floor (y -17..-11), 6 to 15 off with clear
  sight, and took no target in 35 s. The other two took one only after drifting below the floor
  (y about -20). They took it on a 10-tick scan, dropped it the next tick and flickered so (vex 21
  first at 19377 ms); vex 23 charged at 30322 ms, at y -22.66. The pack has `must_see` true,
  `max_dist` 70, no `within_radius`, `charge_attack` at priority 4 and no random move, yet the
  vexes drifted smoothly. A vex with no evoker owner seems to keep a target only from inside
  blocks, which no rule found here explains. Sapling's commanded vex keeps its target and charges
  (`vexes.rs`); left as it is until the target keep is found (a debugger on the charge goal,
  148958560, which has no RTTI).
  Eight evokers held 8.5 to 9 off the warded actor, vexes killed each second (`q3evoker`, 2026-11-05,
  bds-q3evoker.txt against sapling-q3evoker.txt): BDS's first casts were 3 summonings and 5 lines (the
  same casting colours as sapling's), so a first-cast summoning is BDS's too (sapling's 1 of 8). After
  that BDS's never cast one spell twice running: every later cast was the other kind, 14 to 28 s after a
  line began and 22 s after a summoning (its 5 s plus the pack's 17 s cooldown); sapling's cast a line
  each 7 s (25 lines, 9 summonings against BDS's 9 and 7). bds-evoker.txt's lone evoker had cast lines
  7 s apart, so the rule depends on something this drill changed (eight casters, or their vexes dying);
  not fitted.
  The commanded vex (checked 2026-11-05, twelfth wave): this BDS's vex pack has `charge_attack`
  (priority 4) and no `vex_random_move` or `vex_copy_owner_target`, the same for a summoned and an
  evoker's vex, so the difference is in the goal's code, likely an owner test. The goal has no RTTI
  or label (`bdsre find ChargeAttack` finds only its definition's JSON glue, 148958560), and the
  string's seven users are pack readers; the canUse was not reached. Left.
  Rechecked 2026-11-05 (group 8) against bds-vex.txt: BDS's summoned vex took the actor as `TargetEid`
  0.3 s after it came and let it go at 18.8 s, never hit it, and in that time flew straight off
  south-west away from it at a rising 0.4 a tick, so it had a target but no charge; the charge goal's
  code (no RTTI) still was not found. Left as it was.
  Drilled 2026-11-05 (twelfth wave, group 11; `g11vex`, four vexes summoned 6 off the actor, 30 s):
  bds-g11vex.txt had no charge, and the vexes' `TargetEid` flickered to the actor for one tick every 10
  ticks in bursts of a few seconds while they wandered up to 30 off; bds-g11vex-2.txt had the same
  flicker until one vex held the actor from 32.8 s and charged three times (`charge`), hurting it twice.
  So a commanded vex does charge, once it keeps its target; what drops the target the tick after it is
  taken (the pack's `must_see`, with the actor's view of it blocked by the stone floor, is the likeliest)
  was not settled in three BDS runs (bds-vex.txt, bds-g11vex.txt, bds-g11vex-2.txt). Sapling's
  commanded vex keeps its target and charges within seconds. Left, as more runs are needed to fit it.
  Settled 2026-11-07 (thirteenth3ap, g11vex rerun under gdb on the Linux BDS: bds-ap3vexgdb.txt with
  bds-ap3vexgdb-hits.txt and bds-ap3vexrange-hits.txt): the drill's actor fell to the flat ground (y
  -60) before its floor at -20 was laid (`gamemode survival` came first), so the vexes above the floor
  could not see it through the stone and the ones that drifted under it could, 35 to 40 off. Sight is
  the ordinary ray (`Sensing` 0x7266e30, `Actor::canSee` 0xb2fb860, from the vex's eyes to the
  player's position, which is its eyes). `NearestAttackableTargetGoal` (RTTI on Linux; canUse 0xbdd62f0,
  findTarget slot 13 0xbdd7710, target test 0xbdd87a0) takes prey within `max_dist` 70 on its 10-tick
  scan; `TargetGoal::canContinueToUse` (0xbdd6e10) drops it when its position is farther from the vex's
  feet than the follow range attribute, 16.0 here (the pack sets none; read live at 0xbdd6fd7). So it
  flickered past 16 and was kept once vex 23 sank within 16. `ChargeAttackGoal` canUse (0x5769460: a
  held target, a chance, farther than its minimum) never fired on a take tick: 50-odd one-tick takes and
  no charge. Fixed: vex `within=16` (mob-traits), `within` measured from a player's eyes, and a vex
  starts a charge only at prey it took before this tick (`vexes.rs`); test
  `a_commanded_vex_past_16_takes_its_prey_at_each_look_lets_it_go_a_tick_later_and_never_charges`.

## Namespaced summon events (2026-11-05, thirteenth3ah)

`/summon slime <pos> <yaw> <pitch> minecraft:spawn_medium` runs no event on BDS: the slime kept its base
components (a 2.08 box, no `area_attack`, `Variant` 1) and never hurt the actor (bds-q3crowd.txt);
`spawn_medium` made a medium one (bds-ah3slime.txt). `minecraft:spawn_adult` on a piglin left it bare,
as an event its pack lacks. Fixed (thirteenth3aj): sapling matches the pack's exact names; a slime or
magma cube summoned by any other event than a size's or `minecraft:entity_spawned` keeps its base (no
size: the 2.08 box, `Variant` 1, no hops, no touch; `slimes/sizes.rs` `keeps_base`), test
`a_slime_summoned_by_an_event_its_pack_lacks_keeps_its_base_and_never_hurts`. The magma cube's base
is assumed from its pack's like shape, not drilled.

## Animals of group G (2026-11-08, fourteenth3g, `g14*` in fourteenth3g_drill.rs)

- Evoker (bds-g14evoker.txt): no retreat. A creative actor 2 to 11 off saw it stroll (0.6, ~0.194 a tick,
  any way); a survival one 5 off saw it cast and stand still between casts. Sapling already did both.
- Panda (bds-g14panda.txt): a lazy one's `AddEntity` movement is 0.07; a playful one rolls (husbandry/rolls.rs:
  `FlagsExtended` bit 3 for 32 ticks, motion told every tick 0.2 along its facing, 0.27 up at 0, 7, 15, 23);
  a worried one shows bit 22 the tick a held cow is 4 to 16 off. Three aggressive bystanders took no target
  as the actor hit a fourth (not shared). Left: the lazy one's lie-downs (`LayingAmount`).
- Bat (bds-g14bat.txt, bds-g14fly.txt): born under a full block it hangs from the start (`RESTING` in its
  `AddEntity`, feet at floor(y)+1-0.9); flying under one it took hold after ~5 s; all four let go the tick the
  ceiling went (creatures/bats.rs). A player within 4 was not drilled (the actor stood 4.65 off).
- Frog (bds-g14frog.txt): croaks `FlagsExtended` bit 37 for 90 ticks, no sound, 10.8 to 14.1 s apart; in a
  glass pen 6 high it leapt only inside (frogs.rs `clear_way`). Eating: it takes a small slime or magma cube
  ~7 ticks after it comes (`mob.warning`), walks 0.0864, tongue bit 38 ~8 ticks at 1.55 off, `tongue` +1 tick
  0.4675 up its head, prey pulled 0.75 to it and gone 9 ticks on (slime ball 391, a froglight); two fed frogs
  bred, one `PREGNANT` ~3 s on, `LAYING_EGG` 41 ticks, `frog_spawn` on water with two `lay_spawn`. Ported by
  fourteenth2v: frogs/eating.rs (the loot where the tongue met the prey, 1.125 up) and husbandry/frog_spawn.rs
  (it walks at 0.1 to an air cell over water within 10 across and 3 up, lays within 1.7, though BDS's began 2.2
  off); test fourteenth2v.
- Bees (bds-g14bee.txt): bees with pollen grew wheat a block under them 0 to 1 with `ParticleCropGrowth` at
  the cell's corner (bees/crops.rs, pack `grows_crop` 10 charges at 0.03); a silk-touched hive's item kept its
  bee in `Occupants` and none came out; a hive over a lit campfire broken by hand let its bee out angry.
  Fourteenth2v: a bee keeps within 22 of its hive (`minecraft:home` restriction_radius) and stays on a flower
  400 ticks, since MoveToBlockGoal (0x5277e80) counts its stay in twentieths; hives let one out at 1 in 20 a
  tick (block-entities.md).

A witch throws as above (`creatures/arrows.rs` `POTION`, its rise fitted to the three throws),
picking the potion by its pack's rule (`creatures/splashes.rs`). A player it hits gets the full
effect (`vitals/splashed.rs`); one near the break gets `1 - d/4` of each effect's time by the box
grown 0.05 (drilled in t2esplash, above); mobs near it get the same (`creatures/afflictions.rs`).
A witch drinks as above (`creatures/drinks.rs`), a check a second (BDS's delays rule out one a
tick), and has the potion's effects. `s3witch` (2026-11-05, bds-s3witch.txt): a free witch on magma,
not burning, drank fire resistance 0.85 s in, then healing (the pack's `on_hot_block`; sapling's
drank nothing, fixed), and walked 0.135 a tick drinking or not: the pack's `speed_modifier` -0.25
slows BDS's not, nor sapling's. A viewer who comes while it drinks gets its AddEntity, then the
potion's MobEquipment (bds-m3witch.txt), as sapling's announcement now holds it (`drinks::held`).

An enderman teleports on its first tick, the tick after a hit and towards prey more than 16 off
for 30 ticks (`creatures/teleports.rs`), and hunts its hitter as above; a player who stares at it
5 ticks takes it as its prey with `stare` and a teleport (`creatures/stares.rs`, the Java game's
cone). It teleports at random a tick in 100 where its brightness is 0.5 or less and in 20 where
brighter under open sky, none where brighter under a roof, and with no prey whenever a timer of a
random 0 to 600 ticks runs out (`TeleportSystem` 0x145ed2fb0, decompiled by fourteenth1o; the
component's dark default and its pack's `light_teleport_chance` and `max_random_teleport_time`;
BDS's came 5.2 seconds apart at midnight; the rates were not re-drilled), and
water it touches costs it 1 by drowning, with a `splash`, and sends it off that tick. Its random
teleports come beside its prey too, as BDS's did (`a3ender`, research/captures/bds-a3ender.txt: held
2.5 off an actor staring at its head, angry, it teleported twice in ~820 ticks, so no Java
stared-at-under-4 rule; calm, 4 in ~820); the rate itself is open (REMAINING4). Rain hurts it
without sending it off, and its `ANGRY` flag is not shown.

A goat rams players and mobs as above (`creatures/rams.rs`), the nearest of either it sees,
charging at most 8 blocks; a mob it meets is hurt and thrown at once by half a hit's throw and the
push, and runs. It charges at where its prey stood as it sounded `pre_ram` and stops within 0.25
of it (bds-goat-cows-findings.md, "The miss"); a baby's ram costs 1 and pushes a fixed 0.468 of a
grown one's at any step (fixed 2026-10-29 from those captures; they had said so, sapling had not
followed). Between rams it takes the nearest it would ram, looking a tick in ten, shown as `TargetEid`
with `mob.warning` a tick later, and lets it go (`TargetEid` 0) over 16 from its feet to the prey's eyes
or as a ram starts (`hold`, test `a_goat_takes_a_player_near_it_between_rams_and_lets_it_go_past_16`).

An evoker casts its fangs as above (`creatures/spells.rs`, `fights=spells` in `mob-traits.tsv`), its
line's way taken as the first fang rises, and strolls between casts; one cast in four, while
fewer than 8 vexes are within 16, summons three (`creatures/vexes.rs`, `Physics::GHOST`), which fly
by the Java game's vex goals: a tick in 7 a charge at prey more than 2 off, 5.5 on easy where
their boxes meet, else a wander up to 7 off where it came; they waste from 30 to 119 seconds on.
It steps away from nobody and its fangs bite no mob; a fang stands on full blocks only. A vex
lets prey farther than 16 go the tick after it takes it and charges only prey it kept a tick
(see "The commanded vex" above, settled 2026-11-07).

A dolphin swims as above (`creatures/dolphins.rs`, `Physics::SWIMMER`, `InLiquid::Swims`) to
spots of water up to 10 off, or at its hitter, which it bites as other hunters do (`provoked`,
`fights=swims`). Its anger passes to dolphins within 16, shows `ANGRY` and ends 25 s after its
last bite (`creatures/dolphins/anger.rs`); on land it hops and dries out as above
(`creatures/dolphins/ashore.rs`, `Physics::DOLPHIN`). It holds 4800 ticks of breath (`breath=`)
and here never runs out; it does not breach, bob under the surface, turn 10 degrees at most,
flock or swim with players, and takes its prey in the hit's tick, where BDS's took it a tick on.

A fish on land hops as BDS's cod did (`creatures/swimming/ashore.rs`, research/captures/bds-flop.txt),
and is heard `flop` a tick after each hop from where it hopped, as bds-t2water.txt's cod was
(test `a_cod_on_land_is_heard_to_flop_a_tick_after_each_hop`).

A panda is born with a gene by its pack's genetics (`creatures/genes.rs`) and shows it; a weak one
has 10 health, a normal or brown one hits its hitter back once, an aggressive one for 6 until it
is calm, and the rest run. A lazy one keeps the common pace, a worried one keeps from nobody, a
playful one does not roll, and an aggressive one's anger is not passed on.

A mob has effects as above (`creatures/afflictions.rs`): from a witch's potions, splashed or drunk,
from a stray's and a bogged's arrows, and from a plugin (`ShardCtx::give_entity_effect`). The four
newest do as above as it dies or is hurt (`creatures/omens.rs`), to a player too
(`vitals/omens.rs`); the burst pushes no mob or item, and a slime of the second size does not split.
Fire resistance sets and clears the mob's `FIRE_IMMUNE` flag with its `VisibleMobEffects` (one packet), as
BDS's did in `t2flags` (research/captures/bds-t2flags.txt, sapling-t2flags.txt: the same packets); water
breathing sets none (`afflictions/told.rs`). Speed and slowness tell the mob's walking speed in an
`UpdateAttributes` after its `VisibleMobEffects` (base as `default`, BDS's modifiers 0.2 and -0.15 a
level, operation 2), once more for each taken off (bds-a3fx.txt: a zombie's 0.322, 0.2254, then 0.23
twice; `told::paced`). Levitation, slow falling and jump boost move it (`afflictions/moved.rs`,
bds-a3fx.txt, bds-a3fx2.txt), and strength and weakness scale its hit as a player's (effects.md;
bds-a3fx.txt, bds-a3fx3.txt: a zombie's 3 on normal was 4.9 and 7.37 under strength I and II, 1.9 and 1.02
under weakness I and II). Slowness stops a mob's walk, not its hits (bds-d3slow.txt: a zombie held at
movement 0 by slowness 255 or 6, at 0.0575 by slowness 4, a spider and a vindicator at 255, each
teleported 1.1 off the survival watcher, hit it the tick it came and every 20 ticks while it stayed in
reach); bds-a3fx2.txt's hitless holds had the watcher out of reach. Sapling's hunters do the same.
Effects are not saved with it or told to one who comes.

A shulker opens for its prey and shoots as above (`creatures/bullets.rs`, `creatures/shooting.rs`),
41 ticks apart where BDS's varied. Its bullet (bds-d3shulker.txt) comes hidden: AddEntity flags
INVISIBLE, `disappeared` 0.9 up the shulker, shown the next tick, `reappeared` from its middle the
tick after (flight/vanish.rs). A mob in its way takes 4 and levitation, is thrown 0.4 away from the
shulker and 0.4 up, and turns on it; a hit breaks it, `ParticleCritical` 15 at once and gone the tick
after (creatures/struck_back.rs). Idle, a shulker peeks open 30 for 19, 39 or 59 ticks, one tick in 40
(twelve gaps averaged 42); one whose floor goes hops to a free cell over a solid block within 8,
`ShulkerAttachPos` and `teleport` where it stood (`creatures/shulkers.rs`; BDS moved it a tick later).
A bullet is told the pitch and yaw of its last step (bds-d3shulker.txt: 881 of 883 told turns fit
`atan2(dy, across)` and `atan2(-dx, dz)`, cut toward zero to the byte; `creatures/bullets.rs`). A
hurt shulker does not hop: four shulkers hurt 3 a second ten times by `/damage`, open or shut, took
40 hurts with no move (bds-tt1shulker.txt), where the Java game's hops one time in four under half
health; sapling's does not either. One hurt by a player was not drilled.

A breeze faces, shoots, leaps and walks as above (`creatures/breezes.rs`, `creatures/charges.rs`,
`Physics::BREEZE`); it leaps over nothing in its way. Its charge (bds-d3breeze.txt) comes hidden
as a shulker's bullet does; one that meets a mob bursts there at once, 1 to it, thrown 0.4 away from
the breeze and up plus the burst's push; a burst pushes every mob near, its breeze too, and the tick
after swings the wooden doors, trapdoors, fence gates and levers and presses the buttons its power-3
rays reach (an iron door stayed; `blocks/gusts.rs`). A hit turns a charge, as a fireball. Facing, the
breeze turns 30 degrees a tick towards its prey (`legs::face`). Its leap (2026-11-05, bdsre 149fb7240,
14890ff00, 149fb78e0, 14890f450; `creatures/breezes/leap.rs`): only with prey 4 to 20 off and 4 air
cells over it; a spot 2 to 2.83 off the prey along its head yaw + 180 + 45 x a normal draw degrees,
snapped to a floor within 10, not lava, seen from its eyes; the aim that cell's middle pulled 0.5
towards the breeze by its x for both axes (BDS's slip), one above the floor; the first of 40, 55, 60,
75 and 80 degrees shuffled whose parabola, v² = |h² 0.08 / (2 sin cos h - 2 dy cos²)|, needs 1.4 or
less. All five bds-breeze.txt launches fit to 1e-4. [U] the pack's 4-8 landing distance against the
2 to 2.83 the capture fits; the solid flag read as `is_solid`. Jump boost (149fb78e0, effect 8) adds 0.1 a
level times the launch's y over its length to its y, after the solve: bds-g10breeze.txt's three launches under
jump boost V fit to 1e-4 (`breezes/leap.rs` `boosted`). A leap keeps its launch speed across from its first tick
(BDS's x stayed -0.5035 a tick); sapling's took ground friction on its second, fixed in `body.rs`
(sapling-g10breeze.txt). [U] sapling's first SetEntityMotion comes a tick after BDS's. The breeze drill
cannot judge the landings: its capture lacks the actor, whom each burst pushes. A charge that meets
nothing flew on out of sight (62 blocks); sapling's goes after 10 seconds.

A ghast floats and shoots as above (`creatures/ghasts.rs`, `creatures/fireballs.rs`,
`Physics::FLYER`, `blast::explode_fireball`). A spot its box would not reach, there or a block at a
time along the way, is given up (the pack's `float_wander` `must_reach`; not drilled), and slowness
scales its push, 0 at 255 (bds-ghast.txt). Its fireball (bds-d3ghast.txt, bds-d3ghast2.txt) comes
hidden and alight (flags INVISIBLE, then ONFIRE the same tick), pushed off its ghast by the Java
game's `pushAwayFrom` and the ghast back a tick later; a mob it meets takes 6 and burns at once, the
blast the tick after; with mob griefing the blast lights about a third of the free cells over solid
blocks, the rest written again as air. A hit, a player's or a shot's, turns it at the hitter's look
at 1 a tick, the hitter its owner and moved so that tick (one turned back killed its ghast); a blue
wither skull and a breeze's charge the same (bds-d3wither.txt, bds-d3breeze.txt; one path,
`creatures/struck_back.rs`). A player it meets pushes it as it stands (bds-tt1ghast.txt, above). A
fireball that meets nothing goes after 10 seconds.

## Mob knockback and group J (2026-11-08, fourteenth3j, `j3*` in fourteenth3j_drill.rs)

- Mob knockback (bds-j3knock.txt): a zombie's hit on a villager is told in the hit's tick, `HurtAnimation` then
  SetEntityMotion (0, 0.4, ±0.4), then 0.3136 as moved a tick on; an iron golem's six on a zombie went 0.515 to
  0.518 across and 0.82009906 up, the same way (a wolf's bite: bds-g14wolf.txt). Ported for every hunter
  (hunting/prey.rs `hit`, `hits::throw_at_once`). A player's knockback was already told at once. Left: a mob
  that moves after its hitter in sapling's tick is told moved in the same tick too (sapling-j3slime.txt).
- Slimes (bds-j3slime.txt): a large one took an iron golem as its target, its touch hurt it 4 (no throw) with
  `attack` a tick on; packs: slime takes snow and iron golems, magma cube iron golems. A medium one in lava died
  and split 4, which rose 0.02 to 0.04 a tick, hurt by turns; sapling's sank. Ported (fourteenth3p): the
  pack's `slime_float` in lava, `v' = 0.5 v - 0.02 + 0.04` on a 0.8 chance a tick at any depth
  (creatures/floating.rs `IN_LAVA`); the "hurt by turns" was resistance 255 BDS's small ones took (success 4).
  A large one's aggressive hop went 0.193 across; sapling's 0.086. Despawning undrillable by summon (persistent).
- Wind-charged burst (bds-j3omen.txt): four cows fit `1 - d / 7.5` within 0.004, d from the burst to their feet,
  towards 0.9 of their height, added to their motion (-0.0784 standing) and told at once; none at 7.07 or
  beyond. Items went about 0.95 of that, told as moved. An oozing slime leaves only its split; an oozing
  zombie's two size-2 slimes split when killed.
- Creaking heart (bds-j3creak.txt): a shrugged-off hit grows 2 or 3 resin clumps a tick on (the Java game's
  `spreadResin`; face bits down 1, up 2, south 4, west 8, north 16, east 32), each heard `place` at the cell's
  corner, and from a tick after that `block.creaking_heart.trail` every 10 ticks, 10 times, at
  `0.2 + 0.08 k` of the way from the creaking's middle to the heart's, `extra_data` `1000 (2 f - 1)` in f32
  (-600 ... 840). A hit during a trail sets off nothing. Moved 30 off it stayed; 40 or 50 off it was torn down
  at once (16478 a tick on, `death` and state 4 the next, gone the next; no state 3): sapling uses 34.
- Riding (bds-j3ride.txt): a zombie horse's rider took the actor as target ~7 ticks after spawning, steered the
  horse to it and hit (`KineticDamageDealt`); a chicken jockey's baby zombie steered and hit every 20 ticks.
  Ported (fourteenth3p, creatures/jockeys.rs): a seated mob thinks (hunts and strikes); its mount walks to the
  prey at its movement times `mount_pathing`'s multiplier (the chicken's 1.5: 0.3035 a tick, as BDS's), or
  `use_kinetic_weapon`'s `hijack_mount_navigation` pace for a spear rider (zombie horse 1.4: 0.275, as BDS's).
  A chicken takes a baby undead seated at 0.48. Left: the spear charge itself (approach, pass, reposition,
  `KineticDamageDealt`), on foot or mounted; a spear rider hits as a melee one. A mob-ridden mount splashed as an unridden one (splash-sound.md); its rider splashed 0 a tick on.
