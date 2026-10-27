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
slot 16 jumps to the copper routine; not tried live); the `Animal` kinds sapling lists; BDS turns a
pig or villager 0.5 s after the bolt, sapling at once; the sky flash (Weather +0x50) is not sent.
Left out: the skeleton horse trap (1 in {134, 67, 45} by difficulty, 0x142cb06f0), a trident
hitting a rod, items struck, a charged creeper kept in the world folder.

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

Unverified: the bar's title (`entity.wither.name` here); which cell is the spawn origin (the stem's
foot here); the side heads' wait between idle looks (Java's 10 + rand 10); aiming at the target's eyes
less 0.5; its flight (Java's way of keeping 5 over its prey at 0.3 a tick; BDS sets movement 0.6).
Left out: the armoured dash (20 ticks aiming, 10 at 2 a tick, 15 to what it meets), the 200-tick
death and its power-7 blast (it dies as other mobs do, dropping its nether star by `mob-loot.json`),
fire rate halving each sixth of health lost, a blue skull's `max_resistance` 4, skulls meeting mobs,
`ShieldHealth`, `firerate` and `lastHealthInterval` (the charge, `Phase`, armour and `maxHealth` are
kept: research/captures/bds-keepmobs.txt; a summoned wither charges as a built one, at its difficulty's
share of the pack's 600), the soul particles of building, a lying T.

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
way or the attached id); the crossbow's turn angle (Java's 10°); a rocket loaded from the offhand
only; duplicate diamonds or glowstone in a star (sapling refuses them, BDS may not); what a rocket
does when it meets a block or an entity (BDS's hit system notifies the block; sapling stops it at
the block).
Left out: the elytra boost (`FireworksItem::use` 0x1436fdbd0: rocket attached to the gliding player,
a `MovementEffect` glide boost for its life, `v += (look*1.5 - v)*0.5 + look*0.1`) — sapling has
no elytra flight to boost; the dispenser's own call.
