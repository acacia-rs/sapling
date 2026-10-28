# What a click does to a block

Measured 2026-10-11 with `bds_command_probe clicks` (`crates/sapling/examples/shared/use_drill.rs`):
the actor, in creative and looking east, clicks the top of a block a step east of it, with the
hand empty or holding what the block takes. Captures: `research/captures/bds-clicks.txt` and
`sapling-clicks.txt`. Sapling's side is `blocks/tools.rs` (an item on a block), `blocks/handled.rs`
(an empty hand) and `campfires.rs`.

## What BDS did

| Block | In the hand | What happened |
| --- | --- | --- |
| dirt | hoe | farmland, `item.use.on` at the block's middle naming the new block |
| coarse dirt | hoe | dirt, the same sound |
| grass block | shovel | grass path, the same sound |
| oak log | axe | stripped log keeping its axis, the same sound |
| waxed copper | axe | copper block, `ParticleWaxOff` at the middle before the block |
| exposed copper | axe | copper block, `ParticleScrape` |
| copper block | honeycomb | waxed copper, `ParticleWaxOn` |
| pumpkin | shears | carved pumpkin facing south, one pumpkin seeds, `pumpkin.carve` |
| full beehive | shears | honey level 0, three honeycombs one by one, `block.beehive.shear` at the far corner |
| full beehive | glass bottle | honey level 0, `bucket.fill.water` at the far corner |
| cauldron | water bucket | water at level 6, `CauldronFillWater` at the new surface |
| cauldron of water | glass bottle | two levels less a click, `CauldronTakeWater` at the old surface, `bottle.fill` |
| respawn anchor | glowstone | a charge more a click, `respawn_anchor.charge` |
| candle | flint and steel | lit, `ignite` at the cell's corner; a second click did nothing |
| campfire | beef | `SoundItemframeAddItem` at the cell's corner, a click a piece |
| cake | nothing | a bite more a click, `burp` a tenth under the eyes, in creative too |
| redstone ore | nothing | lit redstone ore |
| ripe berry bush | nothing | growth 1, two berries one by one, `block.sweet_berry_bush.pick` |
| farmland | wheat seeds | wheat on it, `place` |
| jukebox | music disc | `Text` of type `JukeboxPopup`, `record.nowPlaying` with `%C418 - 13`, to the clicker; the next click threw the disc out 1.71 over the jukebox's floor |
| obsidian | flint and steel | `ignite` at the middle of the cell over it; no fire stayed |

A cauldron's surface is 0.375 over its floor plus an eighth a level; both of its events carry
-1087090343. The carved pumpkin faced south with the carver looking south and looking east, so
the face clicked decides, not the look.

Sapling does all of these the same; 23 of the drill's 30 cases agree packet for packet once
repeats are set aside. Outside creative a tool wears, honeycomb and glowstone are spent, a bottle
becomes a honey bottle or a water bottle and a bucket an empty one: the Java game's rule, since
the drill ran in creative.

## Note block, composter, flower pot

A note block over stone sent `BlockEvent ChangeState` with the note, one higher a click from 1,
and `note` from the cell's corner carrying instrument * 256 + note (257, 258, 259). Sapling
does the same, the note kept in memory; the instrument is told from the name of the block under
it (`blocks/notes.rs`).

A composter clicked with wheat seeds sent `ParticleCropGrowth` three quarters up from the cell's
corner each click, then rose a level with `block.composter.fill_success` (two clicks of six) or
stayed with `block.composter.fill`. Sapling rolls the Java game's chances by item
(`blocks/composter.rs`); a full one gives a bone meal to an empty hand, which was not measured.

A flower pot given a plant had `update_bit=1`; sapling sets it.

## TNT

Measured with `bds_command_probe tnt` (`bds-tnt.txt`, `sapling-tnt.txt`): TNT three blocks east
of the actor on a stone floor, lit by flint and steel, by a block of redstone beside it, and
summoned.

- Lighting sends `LevelEvent SoundIgnite` at the bottom middle of the cell; the block becomes air
  and a `minecraft:tnt` entity appears at the cell's middle, 0.49 up (its box is 0.98), moving up
  at 0.2 and across at 0.02 in a random direction. A summoned one starts the same way.
- It falls by 0.04 a tick, keeps 0.98 of its speed, and on the ground 0.686 across; it does not
  bounce.
- `FuseLength` is told every tick from 79 down to 0, and it bursts the tick after: 80 ticks.
- The blast is `ParticleExplosion` with data 4, a sixteenth of the box above its bottom, then
  `explode`. On stone it broke ten or eleven floor blocks, which left their cobblestone as one
  stack; the block of redstone left itself. Items lying in the blast were removed.
- Both creative players, 2.9 blocks off, were sent `SetEntityMotion` (-0.56, 0.22, 0.01).

Sapling does the same (`tnt.rs`, `creatures/blast.rs`, `blocks/blown.rs`). Not matched: the
throw's exact size (sapling's, by the Java game's rule, is about (-0.53, 0.29) there), and the
`MOVING` flag BDS turned on and off. TNT a blast reaches is primed on the Java game's short fuse
of 10 to 30 ticks, which was not measured.

## Blasts

Read off BDS 1.26.52.3 with bdsre (`Explosion::explode` at 140795690, `getSeenPercent` at 141602290,
the drop roll in 141a23c30) and measured with `bds_command_probe blasts` and `creeperdrops`
(`bds-blasts.txt`, `bds-creeperdrops.txt`):

- What a block withstands is its own `Block` field (+0xac), read out of a running BDS into
  sapling-data's `block-resistance.tsv`: stone 6, dirt 0.5, obsidian 1200, water and lava 100. A
  ray loses `(resistance + 0.3) x 0.3` in each block and 0.225 a step; air costs nothing. A ray stops
  at water (a block or a waterlogged one), and a blast whose middle is in water breaks nothing.
- Each broken block leaves its items by a chance of one in the blast's power, except TNT's and a TNT
  minecart's with `tntExplosionDropDecay` off, which leave all. The creeper (power 3) broke 77 dirt
  blocks over three blasts, which left 29; TNT with the rule on broke 64 for 13, with it off 64 for 64.
- Like items are merged into stacks of at most 16, 16 more for every five stacks already made, each
  falling out of the cell whose block started it.
- A mob within twice the power is measured from its feet, less half its width. Its impact is
  `(1 - distance / (2 x power))` times the share of points through its box that see the blast (a
  grid of `1 / (2 x size + 1)` steps, 0.1 up from its bottom; all of it when the blast is inside),
  and it is hurt by `(impact^2 + impact) / 2 x 7 x 2 x power + 1`: at least 1 however hidden.
- The hurt throws it as a hit from the blast does: 0.4 across and 0.4 up. Unless no hit throws it, it
  is then pushed by `impact` along the way from the blast to 0.9 of its height: a pig 4.87 off in the
  open went (0.838, 0.467, 0.027), and pigs behind a wall one high and three high only (0, 0.4, 0.4).
- Primed TNT is pushed away by its impact too.

Sapling does the same (`creatures/blast.rs`, `creatures/blast/rays.rs`, `blocks/blown.rs`). Not
done: which order BDS's set of broken cells is walked in (stacks start in another cell), arrows,
orbs and falling blocks are not pushed, and Blast Protection does not shorten a mob's push.

## Cauldrons

`bds_command_probe cauldrons` (`bds-cauldrons.txt`, BDS 1.26.52.3), the actor in survival. A potion cauldron is
`cauldron_liquid=water` with the potion in its entity: `PotionId` (the potion's data) and `PotionType` (0 drunk, 1
splash, 2 lingering); a dyed one has `CustomColor`. Every event is told at the surface (0.375 over the floor and an
eighth a level), every sound at the block's middle.

| Cauldron | Hand | BDS |
|---|---|---|
| empty | potion of swiftness | level 2, `PotionId` 14, `CauldronFillPotion` at the new surface with the potion's colour, `bottle.empty`, a glass bottle back |
| swiftness | potion of swiftness | two levels more |
| swiftness, 4 | potion of slowness | empty, `PotionId` -1, `PotionType` 0, `CauldronExplode` at the floor with water's colour, `bottle.empty` |
| swiftness, 4 | glass bottle | two levels less, `CauldronTakePotion` at the old surface, `bottle.fill` |
| swiftness, 2 | splash potion of swiftness | level 4, `PotionType` 1 |
| swiftness, 4 | 16 arrows | level 3, nothing told but the block |
| water, 6 | red dye | `CustomColor` the dye's colour (0xFFB02E26), `CauldronAddDye` at the surface, level kept |
| red water, 6 | leather cap | level 5, `CauldronDyeArmor` at the new surface with the colour |
| water, 6 | the red cap | level 5, `CauldronCleanArmor` at the new surface with water's colour |
| water, 5 | red shulker box | level 4, `CauldronCleanArmor` |
| water, 4 | plain red banner | nothing |
| swiftness, 6 or 2 | bucket | nothing at all (`bds-cauldrons2.txt`, as the rows below) |
| red water, 6 | blue dye | `CustomColor` 0xFF763968, each channel the mean of the old and the dye's |
| that water, 6 | bucket | empty, `CustomColor` gone, `CauldronTakeWater` at the old surface with the colour |
| water, 6 | white banner with one pattern | level 5, `CauldronCleanBanner` at the new surface with water's colour; clicked again, nothing |

Rain (`bds_command_probe cauldronrain`, `bds-cauldronrain.txt`, `randomtickspeed` 100): an empty cauldron under the
sky filled a level at a time, six levels in 2.3 s starting 0.9 s after the rain, each with the block and its entity
sent again. Sapling fills a level at each of the rain picks of snowfall.rs that lands on one; BDS's rate is not
measured further. Snow fills one with powder snow instead, by BDS's `handlePrecipitation` (BDSRE 141a46850): where
the temperature is under 0.15 the fall is snow; none falls into a full cauldron, lava, a potion, dyed water, or
(once it holds any) snow into water and rain into powder snow. Not drilled on BDS.

`bds_command_probe cauldrons3` (`bds-cauldrons3.txt`):

| Cauldron | Hand | BDS |
| --- | --- | --- |
| empty | powder snow bucket | `cauldron_liquid=powder_snow`, level 6, level event 3514 at the full surface with water's colour, a bucket back (BDSRE 141a415e0: an empty cauldron or one of powder snow) |
| powder snow, 6 | bucket | empty, level event 3515 at the full surface, a powder snow bucket |
| powder snow, 6 | glass bottle | nothing |
| powder snow, 6 | water bottle | empty, `CauldronExplode` at the floor with water's colour, no `bottle.empty` |
| water, 3 or 6 | potion of swiftness | empty, `CauldronExplode` at the floor, `bottle.empty` |
| red water, 3 | potion of swiftness | empty, `CustomColor` gone, `CauldronExplode` with the dye's colour |
| swiftness, 2 | water bottle | empty, `CauldronExplode`, no `bottle.empty` |
| water, 4 | powder snow bucket | empty, `CauldronExplode` at the floor |

Sapling does the same (`blocks/cauldrons.rs`), tipping arrows into `arrow` of the potion's data plus one, as BDS
names tipped arrows.

## Redstone ore

Measured with `bds_command_probe ores` (`bds-ores.txt`): of 25 lit redstone ores set at once, 16
had gone out, to plain redstone ore, after 90 seconds, the first after 0.9 seconds and the rest
spread evenly: each has the same chance every tick, which fits an average life of some 88
seconds. Sapling gives each lit ore a life drawn that way (`blocks/ores.rs`).

`bds_command_probe oresteps` (`bds-oresteps.txt`): ore set under a standing survival player
was lit a tick later, both times, and an ore the player started to break by hand was lit with
the first `BlockStartBreak`. Sapling lights the ore a survival player stands on and the ore a
break starts on; mobs and creative players light none by standing.

## A second round

Measured with `bds_command_probe clicks2` (`crates/sapling/examples/shared/click_drill.rs`;
`bds-clicks2.txt`, `sapling-clicks2.txt`), the same way as the first:

| Block | In the hand | What BDS did |
| --- | --- | --- |
| full composter | nothing | level 0, a bone meal, `block.composter.empty` at the middle |
| cauldron of water, full | bucket | empty, `CauldronTakeWater` at the old surface |
| cauldron of lava, full | bucket | empty, level event 3513 at the old surface |
| cauldron | lava bucket | lava at level 6, level event 3512 at the new surface |
| cauldron | water bottle | two levels more a click, `CauldronFillWater` at the new surface, `bottle.empty` at the middle |
| cauldron | powder snow bucket | `CauldronExplode` low in the cauldron, which stayed empty (not so in `bds-cauldrons3.txt`, a capture after sapling c180cee5: see "Cauldrons") |
| dirt | water bottle | mud, `bottle.empty` at the cell's corner |
| grass block | hoe | farmland, as dirt |
| campfire | shovel | `extinguished` set, `extinguish.fire` at the corner |
| campfire, out | flint and steel | lit again, `ignite` at the corner |
| cake | candle | candle cake, `cake.add_candle` at the corner |
| end portal frame | eye of ender | its eye set, `block.end_portal_frame.fill` at the middle of its top |
| crimson stem, bamboo block | axe | stripped, `item.use.on` |
| cave vines with berries | nothing | a glow berry, `pick_berries.cave_vines` at the corner |
| dragon egg | nothing | gone, `ParticleDragonEggTeleport` at the corner with the jump in its data: a byte for each length and a bit for each that is negative (15 east and 9 north was 68091913) |
| decorated pot | stone | `block.decorated_pot.insert` at the corner, a click a stone |
| item frame | diamond | `SoundItemframeAddItem`, then `SoundItemframeRotateItem` a click |
| lodestone | compass | `lodestone_compass.link_compass_to_lodestone` |
| charged respawn anchor | nothing | a blast of power 5 where it stood |

A chiseled bookshelf clicked on its top took no book, and a hoe on rooted dirt showed nothing.

Sapling does the rows down to the dragon egg the same, state for state and sound for sound, but
for the powder snow, which it leaves alone. An egg jumps up to 15 across and 7 up or down to a
free cell, the Java game's reach. A charged respawn anchor clicked outside the nether is gone
in a blast of power 5 at its middle, which breaks blocks and leaves their items as TNT's does; in
the nether it becomes the clicker's spawn (docs/research/player-damage.md).
An item frame keeps the item in its entity as `Item` and turns it by `ItemRotation`, 45 a
click; a decorated pot keeps its stack as `item`, one more a click of the same thing, the
sound's pitch 700 and 500 for a full stack (`blocks/holders.rs`). Both leave what they hold when
broken.

## Scaffolding

Measured with `bds_command_probe scaffold` (`bds-scaffold.txt`, `sapling-scaffold.txt`): a column
two high on stone, a row set out from its top a block a second, then the column's foot set to
air.

- A block is set with `stability` already told, the column's 0 and the row's 1 to 6, and
  `stability_check` 0; two ticks on it is sent again with `stability_check` 1.
- The seventh and eighth of the row were set with `stability` 7 and two ticks on were falling
  blocks, which landed on the floor as scaffolding of `stability` 0.
- With the foot gone the column's top took 2, one more than the block beside it, and tick by
  tick each block took one more than its nearest neighbour, until each in turn reached 7, was
  sent so with `stability_check` 1, and was air, leaving its item: seven items.

Sapling keeps the same reckoning (`blocks/scaffolds.rs`): none over a solid block, the same as
the scaffolding under it, one more than the nearest beside it; at 7 a block that has not looked
yet falls and one that has breaks. Its capture has the same states in the same order, but that
a block is first shown as set (the default state, `stability` 7) and not with its distance, and
that a block about to break is not shown at 7 first.

## Stacking

Measured with `bds_command_probe stacks` (`bds-stacks.txt`, `sapling-stacks.txt`): the actor, in
creative on grass, puts a thing on the ground a block east of it and three more of the same
onto it.

| Thing | What BDS did with each one more |
| --- | --- |
| candle | `candles` one more, to 3; `item.use.on` from the middle of the cell over it, naming the new block |
| sea pickle | `cluster_count` one more, to 3; the same sound |
| snow layer | `height` one more, to 7, which is eight layers and still a snow layer; the ninth went on top as a new layer (`bds-snowpile.txt`); `place` from its middle |
| pink petals, wildflowers, leaf litter | `growth` one more, to 3; `place` from the cell's floor |
| turtle egg | `turtle_egg_count` one more, to four eggs; `place` |
| oak slab | a double slab at the second, `item.use.on` from its middle; the third went on top as a slab |

Sapling's capture has the same states step for step (`blocks/stacking.rs`). Its sounds differ in
two ways: the first candle, pickle and snow layer are told by `place` where BDS told them by
`item.use.on`, and petals, wildflowers and litter are heard from the cell's middle, not its floor.

## Left

- From the second round: a compass linked on a lodestone is a lodestone compass by name and
  points nowhere. A pot gives its stack back only when broken.
- A frame's item comes out at the first blow, with `SoundItemframeRemoveItem` at the corner,
  and the frame stands (`bds-bonemeal2.txt`, its last steps); sapling does the same, and in
  creative breaks the frame with the same blow, which was not tried on BDS. A respawn anchor clicked with a block in the hand has the
  block put against it; in the nether it sets no spawn.

- Bone meal has its own drill and notes: `bone-meal.md`.
- Fire has its own drill and notes: `fire.md`. The obsidian row above was taken with
  `dofiretick` off.
- A jukebox's music: BDS sent no sound packet for it, and sapling keeps the disc in memory
  (`blocks/jukebox.rs`).

## A comparator behind a jukebox

Measured with `bds_command_probe discs` (`bds-discs.txt`, `sapling-discs.txt`): a comparator
behind a jukebox gives a signal by the disc in it, and none once the disc is out. 13 gave 1, cat
2, blocks 3, wait 12, pigstep 13, otherside 14, 5 gave 15, relic 14, creator 12, its music box
11, precipice 13, tears 10 and lava chicken 9: the Java game's numbers, which sapling uses for
the eight discs not drilled too. Sapling's capture agrees with BDS's for all thirteen.
- A primed note block's instruments past the first four, and a note kept across a restart.
- An empty bucket on a full cauldron, a hoe on rooted dirt, an axe or a shovel on anything not
  listed above: not measured.
