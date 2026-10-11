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

## Left

- Bone meal has its own drill and notes: `bone-meal.md`.
- Fire has its own drill and notes: `fire.md`. The obsidian row above was taken with
  `dofiretick` off.
- A jukebox's music: BDS sent no sound packet for it, and sapling keeps the disc in memory
  (`blocks/jukebox.rs`).
- A primed note block's instruments past the first four, and a note kept across a restart.
- An empty bucket on a full cauldron, a hoe on rooted dirt, an axe or a shovel on anything not
  listed above, a lit redstone ore going out: not measured.
