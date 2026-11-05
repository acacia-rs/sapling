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
Sapling's drill (`sapling-firepace.txt`): fire over stone out at a mean 20.7, 4.5 and 1.0 s, planks'
first change 15.3, 4.1 and 0.9 s, lit ore 1, 9 and 12 of 12 out (a first run had 10 of 12 at 1).
That spread was the drill, not the game: a probe at `TICK_SCALE` 2 counts nominal ticks, so a server
falling behind (both runs were `LAGGED`) gave the ore fewer ticks than BDS's. The shard alone, 12 ores
for 1200 ticks twelve times, put out 4 to 8 (binomial). `bds_command_probe orepace` (three rounds of
63 s at 1): BDS 7, 9 and 10 of 12, sapling at `TICK_SCALE` 1 4, 5 and 6 (`bds-orepace.txt`,
`sapling-orepace.txt`). Pooled over firepace, ores and orepace BDS's lit ore lives some 1400 ticks at 1
against sapling's 1800; that gap is two standard errors. Settled 2026-11-04 by the decompile: a
cell's pace is 1638 to 2048 ticks by its chunk's height, 2048 in a flat drill chunk (growth.md, "The
random tick's pace"); sapling now follows it, fire's spread with it. `orepace` re-run 2026-11-05 on that
rate (twelfth5): BDS 6, 6 and 10 of 12 (`bds-orepace2.txt`; 48 of 72 with the first run), sapling at
`TICK_SCALE` 1 4, 7 and 4 (`sapling-orepace.txt`). The formula at the drill chunk's n = 3 (1755 ticks)
gives 51% out in 63 s: sapling's 42% is a standard error under it, BDS's 67% nearly three over, while
BDS's `ores` (16 of 25 in 90 s), `firepace` and `g3rate` fit it. Left open: no rule found that ends lit ore
faster at rule 1 on BDS; the next step is a lit ore's life read off a patched BDS's random-tick picks.
Decompiled 2026-11-05 (twelfth7): a lit ore's `randomTick` (144bc1f20) puts it out at its first random
tick, with no other path; the picks come from `LevelChunk::tickImpl` (1409f98a0): `floor(2.5 n) x rule`
a subchunk, layer 1 only under a flag, no cell ticked twice. That gives 50% a round (36 of 72) against
BDS's 48, the excess front-loaded (9 out in the first 2 s against 1.6 expected), as if the rule's 0 to 1
step gave more picks. Left open: reading the picks live needs a Linux ptrace port of the
`scripts/bds_dbg*` debugger (Windows only); the probe is a breakpoint after the cell decode in 1409f98a0
and on the Linux build's counterpart of 144bc1f20, counted over the first 2 s after the rule goes to 1.
Left again 2026-11-05 (twelfth10), not cheap: the testbox BDS is a stripped Linux ELF (clang) with no
symbols, so both breakpoints need 1409f98a0 and 144bc1f20 found anew in it by hand (MSVC labels do not carry
over), on top of the ptrace port. A fire aging must not count as a new fire, or it
ticks again a step on: before that fix fire over stone went in 3.0 s at 4.

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

## Lava's fire (2026-11-04, `bds_command_probe lavafire`, `examples/shared/eighth2_drill.rs`)

BDS's random tick (`LiquidBlock::randomTick` 148b5cd10): with `doFireTick` on and air or a block whose
`mLavaFlammable` (Block+0xa6) is set over the lava, up to ten cells within one each way are tried; the
first that is flammable gets fire in the first air beside it, looked at +x, -x, +y, -y, +z, -z [D].
Drilled at rule 40 for 30 s with five pockets, two rows each: lava in a floor hole with a plank
diagonal over it lit the air beside the plank (+x, or -x where +x was stone); a plank lying on the
lava lit the air beside it; lava in a stone ring lit the air over a plank set in the floor beside it;
covered lava lit nothing. Sapling (`fire.rs` `kindled_by_lava`, `growth.rs` `sampled_at`) draws lava
with air or what burns over it, and tries as BDS; test `lava_under_a_plank_lights_fire_beside_it_as_on_bds`.
Drilled on sapling after (`sapling-lavafire.txt`): the same pockets lit and the covered ones did not;
which cell beside a plank caught first differs run to run.
What burns is sapling's flame odds (`fire/burning.rs`), not read off `mLavaFlammable`.

## What lava takes (2026-11-04, `bds_command_probe lavaeats`)

36 blocks were set one by one beside a still lava row in a stone box. BDS's lava took, within 1.5 s:
carpet, moss carpet, repeater, skull, flower pot, end rod, snow layer, torch, rail, web, short grass,
lever, redstone wire, leaf litter, pink petals. It stayed out of: stone button, pressure plates,
standing sign and banner, sea pickle, decorated pot, conduit, candle, ladder, fence, glass pane, iron
bars, cake, heavy core, pointed dripstone, amethyst bud, lantern, lightning rod, trapdoor, brewing
stand. That is BDS's `isLavaBlocking` rule (assumptions-checked.md 4) but for the sea pickle, held out
though its material is open. Sapling: `blocks/liquid/lava.rs`, test
`lava_takes_what_bds_let_it_take_in_the_lavaeats_drill`; `sapling-lavaeats.txt` took the same 15 cells.

## What burns away (2026-11-04, `bds_command_probe burnfloat`)

Seven planks, each with fire beside it and on it a torch, sand, rail, redstone wire, stone pressure
plate, lantern and candle, at rule 40. On BDS, a tick after a plank burnt, the torch, rail, wire,
plate and lantern broke and the sand fell; the candle stayed floating. Sapling settles a burnt cell
as a broken one in the same tick (`fire.rs` `tick`, `updates::settle`), with lanterns now depending
on the block they stand or hang on and candles no longer; test
`what_stood_on_a_burnt_block_falls_or_breaks_as_on_bds`. `sapling-burnfloat.txt` answered every
`/testforblock` as BDS's: the sand down, the candle left floating, the rest air.

## Left

- Burning arrows prime TNT, as on BDS, and light no other block (world-ticks.md "Not drilled").
  Lightning's fire ticks as any written fire does.
- Fire a world folder holds ticks only if its column kept its tick, as on BDS, where `FireBlock`
  takes no random tick and nothing else starts it (vtable 14a8e5550). Sapling saves the queue's
  wait in `PendingTicks` as game ticks at the rule of the save.
- Lava's flag by block name was read off registry call sites, not dumped live; a few sites came out
  unnamed (scratch `flam2.tsv` of the 2026-11-04 decompile), so a single block's flag may be wrong.

## Lava (decompiled 2026-11-04)

`LiquidBlock::randomTick` 148b5cd10 reads only `mLavaFlammable` (Block+0xa6, an enum Never/Always on
`BlockFlammableDescription` beside the flame and burn odds), for the cell over the lava and each tried
cell. The registry 14849c7e0 sets it apart from the odds: Always with no odds for wooden doors,
trapdoors, signs, buttons and pressure plates, the crimson and warped wood family, banners, note
blocks, trapped chests, daylight detectors, mushroom blocks, chorus; Never with odds for the small
flowers, petals, leaf litter, hay, coal blocks, campfires, azaleas and cave vines; a JSON
`"minecraft:flammable": true` is Never. Sapling asked the flame odds; fixed (`fire/burning.rs`
`lava_lights`).
