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

## The snow golem (`snowgolem`)

A snow golem on plains at noon with a zombie 8 off (`research/captures/bds-snowgolem.txt`):

- A layer of snow came the tick after it was summoned under its feet, and as it walked one under
  each of the four cells a quarter block about where it stood at the tick's start, in one update
  where several were new (temperature under 0.81, the pack's `trail`).
- It took the zombie (`TargetEid` the zombie's unique id, `mob.warning` a tick later) once it was
  within 6, walked at it, and threw its snowball 22 ticks after taking it (Java's `RangedAttackGoal`
  throws at 21 with a 20-tick interval), from about its eyes at 1.5 a tick, heard `shoot` from its
  middle. The zombie, which had taken the golem 4 ticks after it came, killed it (health 4) first.

## Drowned with tridents (`drowned`)

A drowned summoned `as_ranged_adult` in a pool 8 from the actor at midnight
(`research/captures/bds-drowned.txt`): shown holding a trident (`MobEquipment`) as it came; it took
the actor and 77 ticks on swung (sapling waits Java's 20 to 60 by distance), a `thrown_trident` following a tick later at 3.94 a tick (the
pack's power 4), heard `item.trident.throw` from the drowned. It kept its own trident; the thrown
one stuck in the floor. One handed a trident by `replaceitem` (an earlier run) only bit.

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
Breaking one is not built; a hung painting is a statue like the stored ones.

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

## Piglins bartering (`barter`)

A piglin in the warded room (`research/captures/bds-barter.txt`) picked a thrown gold ingot up: shown
in its offhand (`MobEquipment` window `Offhand`, slot 1) that tick, `admire` from its head a tick
later, and 161 ticks on (`admire_item` 8 s) the ingot went to its main hand and the offhand emptied.
No barter came out in three runs: the clicks with an ingot fell out of reach. sapling barters on the
click (the pack's `minecraft:interact` with `barter`), with that timing, and drops a roll of
`piglin_barter.json` (read by `mob-loot.py` with the block tables) as the offhand empties.

`piglinkeep` then `piglinkeep2` with `KEEP_WORLD=1` (`bds-piglinkeep.txt`, `bds-piglinkeep2.txt`): a
piglin picked up a thrown golden sword, admired it in the offhand, and 161 ticks on was sent the sword
in its main hand, then the offhand emptied, the same tick; it then took a thrown ingot into the
offhand. After the restart it was announced with the sword (`Damage` 0 in its tag, as thrown) in the
main hand and the ingot in the offhand (slot 0, as a stored mob's hands are shown) and bartered
nothing in the next 11 s. sapling (`creatures/piglins.rs`) keeps both hands with the piglin
(`Mainhand`, `Offhand`; it also writes the armour stand's `Pose`, which BDS's piglins may not have) and
does not resume admiring after a load.

## Sapling

A witch throws as above (`creatures/arrows.rs` `POTION`, its rise fitted to the three throws),
picking the potion by its pack's rule (`creatures/splashes.rs`). A player it hits gets the full
effect (`vitals/splashed.rs`); one near the break gets the Java game's `1 - d/4` of each effect's
time, which was not drilled; mobs near it get the same (`creatures/afflictions.rs`). A witch drinks
as above (`creatures/drinks.rs`), a check a second (BDS's delays rule out one a tick), and has the
potion's effects; it is not slowed as it drinks.

An enderman teleports on its first tick, the tick after a hit and towards prey more than 16 off
for 30 ticks (`creatures/teleports.rs`), and hunts its hitter as above; a player who stares at it
5 ticks takes it as its prey with `stare` and a teleport (`creatures/stares.rs`, the Java game's
cone). It teleports at random a tick in 100 in the dark and in 20 in the sun (the component's dark
default and its pack's `light_teleport_chance`; BDS's came 5.2 seconds apart at midnight), and
water it touches costs it 1 by drowning, with a `splash`, and sends it off that tick. It does not
teleport from beside its prey, rain is not water to it, a carved pumpkin does not hide a stare,
and its `ANGRY` flag is not shown.

A goat rams players and mobs as above (`creatures/rams.rs`), the nearest of either it sees,
charging at most 8 blocks; a mob it meets is hurt and thrown at once by half a hit's throw and the
push, and runs. It charges at where its prey stood as it sounded `pre_ram` and stops within 0.25
of it (bds-goat-cows-findings.md, "The miss"); a baby's ram costs 1 and pushes a fixed 0.468 of a
grown one's at any step (fixed 2026-10-29 from those captures; they had said so, sapling had not
followed). It shows no prey between rams.

An evoker casts its fangs as above (`creatures/spells.rs`, `fights=spells` in `mob-traits.tsv`), its
line's way taken as the first fang rises, and strolls between casts; one cast in four, while
fewer than 8 vexes are within 16, summons three (`creatures/vexes.rs`, `Physics::GHOST`), which fly
by the Java game's vex goals: a tick in 7 a charge at prey more than 2 off, 5.5 on easy where
their boxes meet, else a wander up to 7 off where it came; they waste from 30 to 119 seconds on.
It steps away from nobody and its fangs bite no mob; a fang stands on full blocks only. A vex
spawned by command charges here, where BDS's did not in 30 seconds (`bds-vex.txt`).

A dolphin swims as above (`creatures/dolphins.rs`, `Physics::SWIMMER`, `InLiquid::Swims`) to
spots of water up to 10 off, or at its hitter, which it bites as other hunters do (`provoked`,
`fights=swims`). It holds 4800 ticks of breath (`breath=`) and here never runs out; it does not
breach, bob under the surface, turn 10 degrees at most, flock or swim with players, flop or dry
out on land, or pass its anger on, and its anger does not end.

A panda is born with a gene by its pack's genetics (`creatures/genes.rs`) and shows it; a weak one
has 10 health, a normal or brown one hits its hitter back once, an aggressive one for 6 until it
is calm, and the rest run. A lazy one keeps the common pace, a worried one keeps from nobody, a
playful one does not roll, and an aggressive one's anger is not passed on.

A mob has effects as above (`creatures/afflictions.rs`): from a witch's potions, splashed or drunk,
from a stray's and a bogged's arrows, and from a plugin (`ShardCtx::give_entity_effect`). The four
newest do as above as it dies or is hurt (`creatures/omens.rs`), to a player too
(`vitals/omens.rs`); the burst pushes no mob or item, and a slime of the second size does not split. Their flags (fire immune, breathing) and speed's movement
attribute are not shown, they are not saved with it or told to one who comes, and strength,
weakness, levitation, slow falling and jump boost do nothing to it.

A shulker opens for its prey and shoots as above (`creatures/bullets.rs`, `creatures/shooting.rs`),
41 ticks apart where BDS's varied; a bullet is not shown hidden on its first tick nor turned, hits
no mob and cannot be shot down, and a shulker neither peeks idly nor teleports.

A breeze faces, shoots, leaps and walks as above (`creatures/breezes.rs`, `creatures/charges.rs`,
`Physics::BREEZE`); its leap aims to land on its prey, it leaps over nothing in its way, and its
bursts push no mob, toggle no block and do not push the breeze. A charge that meets nothing flies on.

A ghast floats and shoots as above (`creatures/ghasts.rs`, `creatures/fireballs.rs`,
`Physics::FLYER`, `blast::explode_fireball`); a spot is not checked to hold it, slowness does not
stop it, its fireball is not pushed off it and hits no mob, and the blast lights no fire. A
fireball that meets nothing goes after 10 seconds.
