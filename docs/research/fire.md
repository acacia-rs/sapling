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

Fire then ticks by BDS's rule ("Pace and rule" below). Its age is not sent, as BDS sent none for
a fire that stayed; new fires show theirs, as BDS's did.

## Pace and rule

Settled 2026-11-04 (the seventh wave): the bkfire capture fit the Java rule only loosely and its
wool went the tick it was lit because BDS's first fire tick is 1 queue step after placing; with
`doFireTick` off BDS's fire only reschedules, so it goes out only when it cannot stand.

Decompiled (bds-blocks build, vtable 14a8e5550) [D]: fire waits on the chunk's random-tick queue,
which runs `randomTickSpeed` steps a game tick (growth.md "Where random ticks run"). Placing it
(1471579c0) schedules it 1 step on and turns fire over soul sand or soul soil to soul fire. Its tick
(147157ed0): gone if it cannot stand; with `doFireTick` off only rescheduled; put out by rain over it
or a cell beside it on x or z (biome over 0.15, rain level over 0.2); aged by `nextInt(3)/2` unless
over a block it burns on for ever; rescheduled 30 + `nextInt(10)` steps on. With no block beside it
that catches it goes out over a non-solid block or once older than 3; at age 15 over a block that
does not burn, one in four. The sides are burnt at 300, up and down at 250 (50 less in a biome
with downfall over 0.85): `nextInt(chance) < burn odds` takes the block, becoming fire of age
min(age + `nextInt(5)/4`, 15) if `nextInt(age + 10) < 5`, it is not rained on and something beside
it catches (147159330), else air; TNT is primed. It spreads over dx, dz -1..1, dy -1..4 into air at
rate 100, 100 x dy for dy over 1, when `rate x nextFloat() <= (most catch odds about it + 40 + 7 x
difficulty) / (age + 30)` (halved when humid) and the cell is not rained on.

Measured with `bds_command_probe firepace` (`examples/shared/seventh2_drill.rs`, `bds-firepace.txt`,
2026-11-04): twelve fires over stone, twelve lit redstone ores and twelve lone planks with fire on
them, at `randomTickSpeed` 1, 4 and 20:

| rule | fire over stone out | lit ore out (watched) | plank's first change |
| --- | --- | --- | --- |
| 1 | mean 20.0 s (8.4 to 34.9) | 6 of 12 (60 s) | mean 10.0 s |
| 4 | mean 5.9 s | 8 of 12 (25 s) | mean 3.0 s |
| 20 | mean 1.0 s | 11 of 12 (15 s) | mean 0.8 s |

Both scale with the rule: fire's queue steps, and lit ore's random ticks (block-uses.md "Redstone
ore"). Sapling (`blocks/fire.rs`, `fire/burning.rs`, `random_queue.rs`) does the above, the queue's
steps per world; test `fire_and_lit_ore_go_at_the_pace_of_the_random_tick_speed_as_on_bds`.

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

- Burning arrows light none. Lava does, by BDS's random tick (`LiquidBlock::randomTick`
  148b5cd10, decompiled, not drilled): with `doFireTick` on and air or something that burns over
  it, up to ten cells within one each way are tried, and the first that burns gets fire in the first
  air beside it, looked at +x, -x, +y, -y, +z, -z (`fire.rs` `kindled_by_lava`; sapling draws only
  lava with air over it). Lightning's fire ticks as any written fire does.
- Fire a world folder holds ticks only if its column kept its tick, as on BDS, where `FireBlock`
  takes no random tick and nothing else starts it (vtable 14a8e5550). Sapling saves the queue's
  wait in `PendingTicks` as game ticks at the rule of the save.
- What burns away leaves floating what stood on it.
