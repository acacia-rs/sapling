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
| cauldron | powder snow bucket | `CauldronExplode` low in the cauldron, which stayed empty |
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
in a blast of power 5 at its middle, which breaks blocks and leaves their items as TNT's does.
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

## Left

- From the second round: a compass linked on a lodestone is a lodestone compass by name and
  points nowhere; powder snow on an empty cauldron tells BDS's event and leaves it empty, and
  that the bucket is emptied is a guess. A pot gives its stack back only when broken.
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
