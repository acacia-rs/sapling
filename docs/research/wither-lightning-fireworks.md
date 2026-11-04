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

Unverified: an unwaxed rod that is oxidised seems to be scraped instead of powered (its vtable's
slot 16 jumps to the copper routine; not tried live); the `Animal` kinds sapling lists; the sky
flash (Weather +0x50) is not sent.

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
`a_trap_springs_four_riders_for_a_creative_player_within_ten`), less the enchanting.
Left out: a trident hitting a rod, items struck, a charged creeper kept in the world folder.

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
  hit a blast of power 1, no fire; Wither II for 200 ticks on normal, 800 on hard.
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
Unverified: the bar's title (`entity.wither.name` here); the side heads' wait between idle looks (Java's 10 + rand 10); aiming at the target's eyes
less 0.5; its flight (Java's way of keeping 5 over its prey at 0.3 a tick; BDS sets movement 0.6).
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
(`wither/skulls.rs`, test `a_skull_that_meets_a_mob_blows_up_on_it`). No dash was seen. Left out: the
15 it costs whom it meets (not seen, the actor warded), a blue skull's `max_resistance` 4,
`ShieldHealth`, `firerate` and `lastHealthInterval` (the charge, `Phase`, armour and `maxHealth` are
kept: research/captures/bds-keepmobs.txt; a summoned wither charges as a built one, at its difficulty's
share of the pack's 600), the soul particles of building.

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

Unverified: a rocket with no tag (BDS seems to skip `init` and burst at once; sapling gives it life
10–21); the metadata a rocket is announced with (sapling sends its compound as key 16 only, not the
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
