# Lightning, fireworks and the wither

What sapling copies of BDS for lightning bolts, firework rockets and the wither, and where each rule
came from. Build read: bdsre's index of the test BDS (`6ab54bee`, the `bds-blocks` copy); behaviour
pack entities from `behavior_packs/vanilla/__brarchive/entities.brarchive`. Function names are
bdsre labels or inferred from LeviLamina layouts; addresses are of that build.

## Lightning

Code: `crates/sapling-shard/src/lightning.rs`, `lightning/{storms,rods}.rs`, `creatures/struck.rs`,
`vitals/struck.rs` (`electrocuted`).

- **Storms** (`LevelChunk::tick` 0x1409f98a0, roll at 0x1409fab6d): each ticking chunk of a world
  where it thunders (`rain*thunder > 0.9`) has `nextInt(100000) == 0` a tick; the column is random
  in the chunk, y the top rain-blocking block + 1. Target order: a lightning rod within ±64 every way,
  nearest first, whose column is open and that rain falls on (0x1409fdbc0); else an `Animal`-type
  actor (ActorType 0x1300) in the chunk at or over the height map, at random (0x1409f8a10); else the
  picked top. It strikes only where the biome rains (downfall > 0, warm enough).
- **The bolt** (`LightningBolt` vtable 0x14a7e78d0, `normalTick` 0x142f981b0): life 2, flashes
  `nextInt(3)+1`. On its first tick: LevelSoundEvent 47 `thunder` and 48 `explode`;
  `onLightningHit` on the block at floor(pos); the copper walk; fire at its cell if air plus 4 tries
  at ±1 each way, only with damage on, difficulty ≥ normal and `doFireTick` (0x142f98f60). Life
  drops 1 a tick; under 1, with no flashes left it goes, else at `life <= -3 - nextInt(12)` it
  flashes again (life 1, flashes - 1, one more fire).
- **Hurting** once, the box x±3, y..y+9, z±3: `Actor::onLightningHit` (0x140d3a310), cause 24
  `Lightning`, 5, then 8 s alight unless fire-immune. A mob whose `damage_sensor` takes `lightning`
  is spared that: pig → `minecraft:pig_zombie` (`zombie_pigman`), villager → witch, creeper
  `become_charged`, mooshroom red ↔ brown (`convert_mooshroom`), all unhurt; a turtle takes it 2000
  times over.
- **Rods** (`LightningRodBlock::onLightningHit` 0x148b5b4f0): `powered_bit` 1, strength 15, a tick
  in 8 that turns both off, LevelEvent 2033 `ParticleElectricSpark`.
- **Copper** (copper routine 0x148b39420): from the block under the bolt, or the block a struck rod
  is fixed to (`facing_direction`), if unwaxed copper: `nextInt(3)+3` times scrape it a stage, then
  walk `nextInt(8)+1` steps, each trying up to 10 random cells at ±1 for unwaxed copper to scrape.
- **Channeling** (ProjectileComponent `mChanneling` +0x1b9, 0x1423fe9f0): a trident that hits an
  entity while it thunders, the entity's column open, calls a bolt at floor(its position) and plays
  184 `item.trident.thunder`.
- **Its first tick, drilled** (2026-11-05, thirteenth3m_drill.rs): a summoned bolt (`m3struck`,
  bds-rsdevices.txt) or a Channeling one (`m3rod`, bds-t2efire.txt) is added with a rod's sparks, and
  thunder, the strike, the harm and the rod's power come a tick later; a storm's bolt does it all in the
  tick it appears. Every item (netherite and the nether star too) and orb in the hurting box went in the
  strike tick, before the thunder (`m3struck`). A Channeling trident on a rod (`m3rod`): the bolt at the
  middle of the rod's own cell at the rod's y (0.5, -26, -5.5 for a rod at 0, -26, -6), the rod powered,
  one spark, no `item.trident.thunder`. Sapling does so (`lightning.rs` `call_down`, `channel_to_rod`;
  tests `a_bolt_takes_away_the_items_and_orbs_in_reach_netherite_too`,
  `a_channeling_trident_on_a_rod_calls_a_bolt_onto_it_in_a_storm`).
- **Sparks, drilled** (2026-11-05, `o3rod`, research/captures/bds-o3rod.txt): a bolt in a rod's cell
  gave 1 spark, one block over it 3 to 7, two over none. Each block the hook and the copper walks
  land on that hooks sparks once at its corner, in the bolt's `AddEntity` tick: the rod always
  (waxed too), copper only if its stage was above 0. A storm's bolt drawn to a rod goes into the
  rod's own cell (bds-m3rod.txt 38781). Sapling: `blocks/copper.rs` `hooks`, `blocks/rods.rs`
  `sparks`/`struck`; tests `a_bolt_in_a_rods_cell_sparks_once_powers_it_eight_ticks_and_scrapes_the_copper_it_stands_on`,
  `a_bolt_over_a_rod_powers_it_with_a_spark_for_each_of_its_walks_calls`.
- **Bolt life, drilled** (`o3rod`): 13 bolts lived 10 to 32 ticks, median 19; the rule above,
  simulated, gives 8 to 41, median 19. Sapling follows the rule.

Unverified: an unwaxed rod that is oxidised seems to be scraped instead of powered (its vtable's
slot 16 jumps to the copper routine; not tried live); the sky flash (Weather +0x50) is not sent.

- **`Animal` kinds, checked (fourteenth K):** 0x1409fae83 passes 0x1300 to 0x1409f8a10, whose test
  0x140d27800 is `(type & 0x1300) == 0x1300`. The 31 ids that pass (LeviLamina `ActorType.h`) are
  lightning/storms.rs's `ANIMALS`, which had 13: added were the ocelot, cat, parrot, the five horse
  kinds, hoglin, strider, axolotl, frog, trader llama, camel, camel husk, sniffer, armadillo and
  happy ghast. `k2bolt` (bds-k2bolt.txt) struck each kind with a summoned
  bolt for the hurt and conversions.

Drilled (2026-11-04, `bds_command_probe n3struck`, research/captures/bds-n3struck.txt): a struck
pig turns 10 ticks after the bolt's strike: `AddEntity` of the zombie pigman, its `MobEquipment`,
`RemoveEntity` of the pig, then `mob.pig.death` heard at the pig's mid height (entity type "", unique
id -1); the new mob makes no `spawn` sound. Sapling now does the same (`creatures/struck.rs`
`Turning`; test `a_pig_becomes_a_zombified_piglin_and_a_creeper_is_charged_unhurt`). A villager 3 blocks off the pig's bolt
turned into a witch in the same tick as the pig, with no sound. `/summon villager_v2` is a syntax
error on BDS; `villager` summons one.
Skeleton horse traps (2026-11-04, `bds_command_probe u1trap`, bds-u1trap.txt): a bolt not drawn to a
rod leaves one 1 in {134, 67, 45} by difficulty with `doMobSpawning` on (0x142cb06f0, table 0x14a7d4940),
a skeleton horse with `minecraft:set_trap` at the floored spot. `/summon skeleton_horse … minecraft:set_trap`
makes one too. It sprang the tick a creative or a survival actor came within 10 (one 40 off waited
60 s): four new skeleton horses where it stood, each pushed off within ±1.15 on x and z, each ridden by a
skeleton in an iron helmet holding a bow BDS enchants at random (unbreaking 1, power 1), a bolt there,
the trap taken away. Sapling does the same (`creatures/skeleton_traps.rs`, `lightning/storms.rs`; test
`a_trap_springs_four_riders_for_a_creative_player_within_ten`). The gear, decompiled
(`SkeletonHorseTrapGoal` 0x149fee7b0): a cost of the regional difficulty's special value plus 5, plus
`nextInt(18 - cost)` below 18, enchants the helmet and the bow at random (`randomlyEnchant`
0x141b62a20); BDS's eight helmets came `ench: []`, its bows unbreaking or power 1 or 2. Sapling rolls
the bow with the enchanting table's draw at that cost and shows the helmet's empty list; each rider is
told in its `AddEntity`, helmet then bow, then the link, as BDS. A stored trap keeps its
`minecraft:skeleton_trap` group and is a trap again when loaded, and a charged creeper its
`minecraft:charged_creeper` group, shown charged again (tests in `kept_groups.rs`).
A trident hitting a rod [D] (`LightningRodBlock` slot 15, `onProjectileHit`, Linux 0xc538e10, the
slot the target, bell and dripstone blocks override too): the projectile's Channeling flag (+0x1b1),
then the weather at the cell above and the sky over the rod, then a bolt; read as Java's (x + 0.5,
y + 1, z + 0.5) with a sound, but drilled in the rod's own cell and silent ("Its first tick, drilled"
above). Sapling: `blocks/target.rs` `shot_stopped`, `lightning.rs` `channel_to_rod`.

## The wither

Code: `crates/sapling-shard/src/creatures/wither.rs`, `creatures/wither/{heads,skulls,wreck}.rs`,
`blocks/summoning.rs`.

- **Building** (`SkullBlock::checkMobSpawn` 0x146601230): placing `wither_skeleton_skull`, not on
  peaceful, finishing rows `^^^`, `###`, `~#~` (`#` soul sand or soul soil); the blocks become air and
  the wither stands at the match + (0.5, 0.05, 0.5).
- **Spawn** (`reloadHardcoded` 0x142f8fbe0, `preAiStep` 0x142f903b0): health 1 of a max (pack 600)
  times 0.5 / 0.75 / 1 on easy / normal / hard; 200 spawning frames told as
  `WitherInvulnerableTicks` (48), healing ceil(max/200) a tick; then a blast of power 7, griefing by
  `mobGriefing`, and sound 53 `spawn`. While charging only the void hurts it.
- **Phase 2**: at health ≤ max/2 `WitherAerialAttack` (52) goes 0 (armoured: arrows do nothing), a
  blast of power 7 from its head, 80 ticks stunned, and on normal and hard 3 wither skeletons.
- **Heads** (`_performRangedAttack` 0x142f95cd0): the middle one shoots its target every 20 ticks,
  every 4th skull blue; on normal and hard each side head takes a random player within 30 across, 8
  up or down, in sight (10 tries), shoots it, then waits 40 + rand; after 15 idle looks head 1
  shoots a blue skull at a random point ±10/±5/±10. Firing points: middle (x, y+3, z), sides y+2.2,
  1.3 aside by yaw + head·180 − 180. Targets told as `WitherTarget1..3` (49–51).
- **Skulls** (pack `wither_skull`, `wither_skull_dangerous`): power 1.2 (blue 0.6), no gravity; on a
  hit a blast of power 1, no fire; Wither II for 200 ticks on normal, 800 on hard. A player's hit
  turns a blue one (`reflect_on_hurt`) along the hitter's look at 1 a tick, the hitter its owner
  (research/captures/bds-d3wither.txt); a black one flew on and burst on the hitter.
- **Block breaking** (`_destroyBlocks` 0x142f92bd0, `canDestroy` 0x142f8f450): 20 ticks after a
  hurt, with `mobGriefing`, its box ± 1, but bedrock, portals and frames, gateways, command blocks,
  barrier, border, allow, invisible bedrock, camera, chalkboard, structure blocks and void,
  reinforced deepslate; sound 55 `break_block`.
- **Bar** (`minecraft:boss`): range 55; colour byte 6 (`RebeccaPurple`). **Regen**: 1 every 20 ticks
  only with no player within ±50. Immune to falls, drowning, and its own blasts.

A T lying on the floor is found too (2026-11-04, `bds_command_probe n2wither`, research/captures/bds-n2wither.txt:
stem west of its arms, the last skull put by the actor); sapling tries the T every way. The spawn was not the
stem's foot: an upright T along z at x 3, foot (3,-26,0), gave the wither at (4.5,-25.95,0.5), and the lying
T, foot (2,-26,0), at (6.5,-27.95,0.5), in the ground. BDS's matcher (0x1415f2940) spawns it from its
match corner, the skull row's lowest cell on each axis, plus (1.5, -1.95, 1.5) in world axes whatever
way the T stands; sapling does so since 2026-11-04 (`blocks/summoning.rs`). That fits the upright T; the
lying one's wither was announced a block further east than the rule gives (left: pushed out of the floor?).
Re-drilled 2026-11-05 (twelfth wave, `bds_command_probe g8wither`, research/captures/bds-g8wither.txt,
sapling-g8wither.txt): skulls along z at x 4 with the stem at x 2 and with it at x 6 both gave the wither at
(6.5, -27.95, 0.5), so it is no push out of the floor (the stem at 6 leaves that cell air) and not the
skull row's corner plus (1.5, -1.95, 1.5) (sapling 5.5); the upright T gave (4.5, -25.95, 0.5) again. The
matcher (0x1415f2940, tried by 1415f25d0 for each forward and up) returns its search origin + ((width - 1)
/ 2, -(height - 1), (width - 1) / 2) in world axes; the origin it is handed for a lying T was not traced
(callers 144bbf720). The two Ts lying along x (stems at z -2 and 2) made no wither on BDS in two runs: the
bot's last skull went astray (`nothing to place against`), so the rule for them is not known. Fixed for
what was seen: a T lying with its skulls along z stands its wither a block further east than the corner
rule (`summoning::lying_east`, test `a_t_lying_with_its_skulls_along_z_calls_it_two_and_a_half_east_of_them`).
`s3wither` (2026-11-05, bds-s3wither.txt): a T lying with its skulls along x at z 0 (x 3 to 5), its
stem north at z -2, gave the wither at (5.5, -27.95, 1.5), the corner rule plus a block east too, so
`lying_east` holds for any lying T (tests in wither_lying.rs; sapling-s3wither.txt the same).
Drilled (`o3wither`, research/captures/bds-o3wither.txt): the bar's title is `entity.wither.name`, as
sapling's. Its flight, fitted tick by tick: v = (v − 0.08)·0.98·0.6; while under its prey's height
+ 5 (its height alone once armoured), v = max(v, 0) + 0.3 − 0.6·max(v, 0); a player's height is its
eyes, a mob's its feet. It settles rising 0.3677 a tick and bobs 0.3, 0.1294, 0.029; its own
charge's blast does not throw it. Sapling: `creatures/wither/flight.rs`, test
`it_rises_to_five_over_its_preys_eyes_and_bobs_there`.
Drilled 2026-11-05 (`y3wither`, research/captures/bds-y3wither.txt: 75 s with nobody to shoot, then a
survival actor held in place 50 s): the idle blue skulls came 231, 242, 264 and 243 ticks apart in one
stream, which fits head 1 alone counting its own idle looks, 10 to 19 ticks apart, and shooting on the
17th; sapling's heads had shared one count and shot far more often. 38 skulls passed the standing actor
a mean 2.41 over its feet (median 2.11, sd 1.15): the aim is half the eye height over its eyes (1.62 +
0.81 = 2.43), a mob's half its height x 0.85. Sapling: `creatures/wither/heads.rs`, tests in
wither_heads.rs. Left: BDS's skulls spread about that aim at random, sapling's fly straight.
Ported 2026-11-05 (thirteenth3ab): both skulls' pack `minecraft:projectile` gives `uncertainty_base`
7.5 and `uncertainty_multiplier` 1, so each part of the one-long aim is off by a normal throw of 0.0075 x
(7.5 less 1 a difficulty level), as mobs' arrows (`creatures/arrows.rs` `spread`; ProjectileComponent::shoot
1424020c0); `wither/skulls.rs`. Test `skulls_are_aimed_half_the_eye_height_over_a_players_eyes_and_spread_about_it`
(was one skull within 0.05 of 2.43, which the capture's sd 1.15 rules out). Not re-drilled.
Its 200-tick death is built as BDS's in `witherdeath` (research/captures/bds-witherdeath.txt,
creatures/wither/dying.rs): `WitherInvulnerableTicks` 199 down to 1; `WitherAerialAttack` (a Short)
flips at 15 ticks and then whenever the last gap less one divides the tick (28, 39, 48, 55, 60, 63,
64, 70, 72, 75, 76, 78, 80, 81, then every tick); `death.min.volume` at once, `death` at 80 and
`death.mid.volume` at 140, 2.55 over its feet with its unique id; at 199 the power-7 blast 2.7 over its
feet (as the charge's), its death shown again and its hands emptied; at 200 its loot and experience.
Its armoured dash, two `witherdash` runs (research/captures/bds-witherdash.txt at the warded actor,
two dashes; an earlier run at an iron golem, not kept, one; creatures/wither/dash.rs): first 197 ticks after the
half-health blast in both, the next 320 later; straight across at where its prey stood, 2 a tick for 9
ticks (told as 1.82, after drag), through it (the actor was thrown, the golem hurt) and on, then
dead still. `t2wither` (2026-11-04, research/captures/bds-t2wither.txt; a wither on normal, a held cow,
the actor warded): volleys of four skulls (three black, then a blue), 20 ticks apart, 140 to 180 ticks
between volleys and no shorter as its health went 300, 252, 203, 155, 97, so no halving each sixth of
health lost (sapling has none). The rest after a volley's blue skull was 60 to 140 ticks in steps of 20
here and in bds-witherdash.txt (3 to 7 of its 20-tick fire rate); sapling's middle head shot every 20
ticks without a rest, now in BDS's volleys (`wither/heads.rs` `RESTS`, test
`its_middle_head_shoots_in_volleys_of_four_then_rests_60_to_140_ticks`). A skull flew at the cow, its prey, and burst on it (`bow.hit`, the
blast, its death). Sapling's skulls met no mob; now they do and give a mob met Wither II as a player
(`wither/skulls.rs`, test `a_skull_that_meets_a_mob_blows_up_on_it`). Its prey and middle head's
target took players only; now the pack's `nearest_attackable_target` (players, and mobs neither
undead nor inanimate, within 70, seen) picks the nearest (`wither/heads.rs` `prey`, test
`its_middle_head_takes_a_living_mob_but_no_undead_one`; 2026-11-05, twelfth wave). No dash was seen. A blue
skull's blast counts no block's resistance above its pack's `max_resistance` 4, 0.8 in sapling's
units (`blast::rays::Capped`; not drilled). Built, BDS showed 120 `ParticleProjectileHit` events
(data 15) the tick the T went, before its blocks' updates and the wither's `AddEntity`, each at the
wither's spot plus a fraction across x and z and 2 below to 1.9 above (bds-gxwither.txt; the Java
skull block's numbers); sapling sends them so (`blocks/summoning.rs` `souls`). Left out: the
15 it costs whom it meets (not seen: `m3wither`'s cow phase ended at 275.76 of 450, above half, so no
dash came), the soul particles of building.
Decompiled (thirteenth3j) [D]: `WitherBoss` load 142f97350 / save 142f96ca0 keep `ShieldHealth` at
+0x474, `firerate` +0x4f0, `lastHealthInterval` +0x518, `SpawningFrames` +0x4e8, `Phase` +0x4c8,
`maxHealth`. Its hurt (142f96660, vtable slot 247): off easy, once health drops below
`lastHealthInterval` less the step at +0x514, `firerate` halves and the interval moves down; the
half-health turn sets `firerate` back to 20 and doubles +0x510. The tick (142f933e0) reloads the
middle head's wait (+0x52c) and the side heads' (+0x4ec) from `firerate`. No code past load and
save reads `ShieldHealth` in the wither's own functions [U: what it guards].
Kept armoured (2026-11-08, fourteenth N, `f14nwither` then `f14nwither2` on the kept world,
bds-f14nwither{,2}.txt): hit to half, saved 60 ticks later (137 before its first dash), it dashed
about 140 ticks after the reload and again about 280 after that, so BDS keeps its dash timer across a
load under no key its loader names. Sapling keeps it as its own `DashIn` (ticks left; `wither.rs`
`adopt`/`keep`); a BDS-saved armoured one with none dashes 197 ticks after it arrives.
Drilled (2026-11-05, `m3wither` and `m3wither0`, thirteenth3m_drill.rs, research/captures/bds-m3wither{,0}.txt,
the worlds dumped with `actor_dump`): `t2wither` ran on easy (warden_drill.rs `start`), hence no halving.
On normal, hurt 10 every 3 s from 450, the volley spacing went 20 → 10 below 375 → 5 below 300: the step
is a sixth of `maxHealth` (75) and `lastHealthInterval` starts at `maxHealth`; the rests after a blue skull
stayed 80 to 120 ticks whatever the rate. Saved: `ShieldHealth` 30 always, `firerate` and
`lastHealthInterval` as they stood (20 and 450 unhurt), `Phase` 1 until armoured, then 0. Sapling does the
same (`wither/heads.rs` `Rate`; sapling-leveldb actor_state.rs `WitherRecord`; test
`off_easy_each_sixth_of_its_health_lost_halves_its_fire_rate_and_is_saved`). Sapling's wither hovered
higher (skulls from y -6 against BDS's -16 to -18 over the actor at -26); fixed by the flight above.

Head aim (`ab1wither2`, research/captures/bds-ab1wither2.txt; fourteenth1ab): the wither among three cows held
by slowness and resistance in the open, the players creative far off. It took a cow (`TargetEid`) as it
spawned. Spawning, body and head spun together, then the body stopped and the head took random looks. Once
it flew, the head yaw told (`MoveEntityDelta` rot_z) stayed within 5 degrees of the yaw to its prey on 483 of
566 told moves, turning 20 a told move (10 a tick) to get there, while the body yaw went its own way (up to
170 off). After a volley's blue skull the head went back to the body for about a second, then turned back
at 10 a tick; the told pitch stayed 0 or 8, not aimed. Sapling's middle head did not turn; now it turns 10 a
tick towards its prey across, or back to the body with none (`wither/heads.rs` `look`; test
`the_withers_middle_head_aims_its_told_head_yaw_at_its_prey`). Not ported: the look back after a volley and
the random looks while spawning.

## Fireworks

Code: `crates/sapling-shard/src/fireworks.rs`, `fireworks/{item,recipes,burst}.rs`,
`shots/crossbows.rs`, `inventory/crafting.rs` (`special`), `sapling_data::is_special_recipe`.

- **Tags** (`initFireworksRocketItem` 0x1436ffc70, explosion builder 0x1436fcaa0): a rocket's
  `Fireworks{Explosions:[...], Flight:byte}`; each explosion `FireworkType` byte, `FireworkColor`
  and `FireworkFade` byte arrays of dye numbers (0 black .. 15 white), `FireworkFlicker`,
  `FireworkTrail`. A star keeps one in `FireworksItem` and a `customColor` int,
  `0xFF000000 | RGB` with each channel the mean of its colours' (table 0x14a7716e0).
- **Crafting**: one multi recipe, `FireworksRecipe`, UUID `00000000-0000-0000-0000-000000000002`
  (0x142cafa40), matcher 0x142c7e480. Rocket: 1 paper, 1–3 gunpowder, any stars, nothing else →
  3 rockets, `Flight` the gunpowder. Star: 1 gunpowder, ≥1 dye, at most one shape (fire charge 1,
  gold nugget 2, a head 3, feather 4), diamond trail, glowstone flicker → 1 star. Fade: 1 star and
  dyes. Sapling accepts any multi recipe id whose grid makes one of these.
- **Flight** (`FireworksRocketActor`, lifetime 0x145987240, move 0x1424decf0, `postNormalTick`
  0x145987410): life `10*(Flight+1) + rand(6) + rand(7)`; it starts at `gaussian*0.001 + way*0.03`
  and each tick goes `v*1.05 + way*0.03`, no gravity; sound 56 `launch` at life 0; past its life it
  sends entity event 25 `FireworkParticles` (the blasts and twinkles are the client's) and goes.
- **Harm**: only with n ≥ 1 stars: `5 + 2n` times `sqrt((5 - d)/5)` to every mob (players too)
  within 5 that a block clip from it reaches, cause 23 `Fireworks`.
- **Use on a block** (`_useOn` 0x1436fd690): at the click, way up; spent outside creative.
  **Crossbow** (0x1436ec1a0): one rocket a Multishot turn along the look, slow start as above,
  3 wear. **Dispenser** (0x1436fe320): out of the face at `0.5 + facing*(0.5 + 0.125)`, y lowered
  0.125 for up and down; `fireworks::dispense` is the dispenser's way in.

A rocket with no tag (bds-n13fw.txt, 2026-11-05, used on a block): it lived 27 and 22 ticks, as one of
Flight 1 (bds-glide.txt's boost of 44 too), and was heard `launch` with no entity type where its first
tick took it, a tick after AddEntity; sapling now does both. BDS put it at the click on the face, not
the face's middle as sapling does, and removed it a tick after the burst, sapling at once [U: not
changed]. Unverified: the metadata a rocket is announced with (sapling sends its compound as key 16 only, not the
way or the attached id); a rocket loaded from the offhand
only; duplicate diamonds or glowstone in a star (sapling refuses them, BDS may not); what a rocket
does when it meets a block or an entity (BDS's hit system notifies the block; sapling stops it at
the block).
Left out: the elytra boost (`FireworksItem::use` 0x1436fdbd0: rocket attached to the gliding player,
a `MovementEffect` glide boost for its life, `v += (look*1.5 - v)*0.5 + look*0.1`) — sapling has
no elytra flight to boost, and the drill bot cannot glide; the dispenser's own call.

Drilled 2026-11-04 (research/captures/bds-t2efire.txt, sapling-t2efire.txt, open ground): a rocket
shot from a crossbow left at (0.5, -58.38, 0.5) at 0.03 east on both; a Channeling trident thrown in
thunder at a cow 6 east put a `lightning_bolt` at (6.0, -60, 0.0) on both. Multishot arrows are
turned 10° (z ±0.545 of 3.1 on BDS) and added left, straight, right (bds-t2emulti.txt); sapling added
the straight one first. Fixed (`enchant/archery.rs` `multishot`).
