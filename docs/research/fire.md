# Fire lit by hand

Measured 2026-10-11 with `bds_command_probe fire` (`crates/sapling/examples/shared/use_drill.rs`),
`dofiretick` on: the actor, in creative on stone and looking east, strikes the top of a block two
east of it and waits. Captures: `research/captures/bds-fire.txt` and `sapling-fire.txt`. Sapling's
side is `blocks/fire.rs` and `strike` in `blocks/tools.rs`.

## What BDS did

| Struck | With | What happened |
| --- | --- | --- |
| stone, netherrack, obsidian | flint and steel | `fire` in the cell over it, then `ignite` at that cell's middle; it burnt on for the six seconds watched |
| soul sand | flint and steel | `soul_fire`, the same sound |
| oak planks | flint and steel | fire, which spread to the cells beside and over the planks; the three planks were gone within twenty seconds |
| white wool | flint and steel | fire; the wool under it was gone the same tick and fire stood beside it |
| stone | fire charge | `LevelEvent SoundGhastShoot`, data 78642, from the lit cell's corner, then fire; no `ignite` |

With `dofiretick` off (the `clicks` drill) flint and steel on obsidian gave the sound and no fire.

## Sapling

Flint and steel and a fire charge light the free cell on the clicked side when a solid block is
under it, with BDS's sound of each; soul fire over soul sand and soul soil. Flint and steel wears
and a fire charge is spent outside creative.

Fire then ticks every 30 to 40 ticks by the Java game's rule and odds (`FireBlock.tick`): it
ages, goes out once old with nothing to feed it (never over netherrack, magma, soul sand or soul
soil), burns the blocks around it away or into fire, lights TNT, and spreads to free cells by
what burns. Its age is not sent, as BDS sent none for a fire that stayed; new fires show theirs,
as BDS's did. `doFireTick` off stops the ticks. In sapling's drill the planks were alight within
two seconds and burning away by eight; BDS's first spread came at 3.7 seconds and the planks
went between 7 and 21.

What burns is told by tag and name with the Java game's numbers (`burns` in `blocks/fire.rs`).

## Putting it out

Measured with `bds_command_probe quench` (`bds-quench.txt`, `sapling-quench.txt`), the actor in
survival and the fire set by `/setblock`:

| What was done | What BDS did |
| --- | --- |
| the fire broken by hand | `BlockStartBreak`, `ParticleDestroy` of the fire at its middle, air; no sound of its own |
| a bucket of water emptied on the block under it | water where the fire was |
| the block under it set to air | the fire gone in the same tick |
| rain, the fire over stone | gone within two seconds |
| rain, the fire over netherrack | burnt on for the twelve seconds watched |

Sapling does the same: fire is a block that needs a solid block under it or one that burns
beside it (`ground.rs`), any fire written ticks, and a tick under rain and an open sky puts out a
fire that does not burn on. It also sends the fire's `ParticleDestroy` when its hold goes, which
BDS did not.

## Left

- BDS's own rule and odds: the capture fits the Java game's only loosely, and its wool went in
  the tick it was lit.
- Fire going out with `dofiretick` off as BDS's did.
- Fire that a world folder holds does not tick until it is written again; lava, lightning and
  burning arrows light none.
- What burns away leaves floating what stood on it.
