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

## Parity drill (2026-10-29, `bds_command_probe bkfire`, research/captures/bds-bkfire.txt)

Lava with air over it in a stone pit, planks around it at each distance, `randomTickSpeed` 30: on
both servers the lava lit fire beside the planks within a few seconds and the planks burned away;
which cells caught and when differ run to run, so the odds are not pinned. A burning shot is in
world-ticks.md "Parity drill".

## Left

- BDS's own rule and odds: the capture fits the Java game's only loosely, and its wool went in
  the tick it was lit.
- Fire going out with `dofiretick` off as BDS's did.
- Burning arrows light none. Lava does, by BDS's random tick (`LiquidBlock::randomTick`
  148b5cd10, decompiled, not drilled): with `doFireTick` on and air or something that burns over
  it, up to ten cells within one each way are tried, and the first that burns gets fire in the first
  air beside it, looked at +x, -x, +y, -y, +z, -z (`fire.rs` `kindled_by_lava`; sapling draws only
  lava with air over it). Lightning's fire ticks as any written fire does.
- Fire a world folder holds ticks only if its column kept its tick, as on BDS, where `FireBlock`
  takes no random tick and nothing else starts it (vtable 14a8e5550). BDS keeps fire on its
  random-tick queue, which runs `randomTickSpeed` steps a tick (see growth.md "Kelp"), so its fire
  ticks faster with the rule; sapling's runs on game ticks, and saves the tick in `PendingTicks`.
- What burns away leaves floating what stood on it.
