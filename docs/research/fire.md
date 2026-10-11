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

## Left

- BDS's own rule and odds: the capture fits the Java game's only loosely, and its wool went in
  the tick it was lit.
- Rain, and fire going out with `dofiretick` off as BDS's did.
- Fire that a world folder holds, and fire from lava, lightning and burning arrows, does not tick.
- What burns away leaves floating what stood on it.
