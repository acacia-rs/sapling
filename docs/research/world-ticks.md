# World ticks: coral, nether vines, chorus, amethyst, sponges, lecterns, burning shots

Read 2026-10-28 from `acacia/.testserver/bds-blocks/bedrock_server.exe` (PE stamp 6ab54bee) with bdsre
(`find|str|refs|dis`; `c` does not run on this Linux box, its radare2 is the Windows build). Nothing here
was drilled. Lightning (wither-lightning-fireworks.md), fire, rain and lava fire (fire.md), copper
(copper.md), ice, snow, mycelium, mushrooms, vines and kelp (growth.md) were built before this pass.

## Which blocks take which tick

RTTI of the `BlockEventExecutor<...>::subscribe<Block, Event>` lambdas says which events each block class
handles. `find BlockRandomTickEvent` lists the random-tick subscribers: bamboo, budding amethyst, cactus,
cave vines, chemical heat, chorus flower, cocoa, copper chest, crops, double sapling, eyeblossom, farmland,
flower pot, grass, ice, leaves, liquid, magma, mangrove propagule, mud, mushroom, mycelium, nether wart,
nylium, pitcher crop, portal, redstone ore, sapling, sea pickle, soul sand, pointed dripstone, stems, sugar
cane, sweet berries, snow layer, vine (and the script registry). Others of note:

| Class | Events |
| --- | --- |
| `WeepingVinesBlock`, `TwistingVinesBlock` | place, queued tick, player interact; no random tick |
| `CoralBlock`, `CoralPlantBlock` | place, queued tick |
| `CoralFan`, `CoralFanHang` | place only (their dying is in a vtable slot, not followed) |
| `ChorusFlowerBlock` | queued tick, random tick; `ChorusPlantBlock` queued tick |
| `SpongeBlock` | place, queued tick |
| `BigDripleafBlock` | place, queued tick, redstone update: no random tick, so dripleaf has nothing to grow by |
| `LecternBlock` | place, queued tick, player interact |
| `TntBlock` | player interact, redstone update |

## Coral (`blocks/coral.rs`)

Live coral of the five kinds (block, plant, fan, wall fan) turns into `dead_<name>` with its properties
at a queued tick if then dry: no water in its cell or the six beside it (water, flowing water or a
bubble column, in either layer). BDS's queue keeps every tick asked for (`add` 1415f50f0); the first
decides. Decompiled 2026-11-04 (bdsre, pseudo-C not kept):

| Coral | Written itself | A neighbour written | At its tick, wet |
| --- | --- | --- | --- |
| block (`CoralBlock` 14713b7c0, 14713bd50) | 45 if dry | `40 + nextInt(160)` | nothing |
| plant (`CoralPlantBlock` 14713e660, 14713ebb0, 14713e950) | `40 + nextInt(160)` if its own cell has no water | 5 | waits `40 + nextInt(160)` again |
| fan, wall fan (14713c770, tick 14713c9c0) | not decompiled; sapling keeps the long wait | `40 + nextInt(160)` | nothing |

Drilled (bds-e3coralmany.txt): four plants set alone died after 87, 141, 58 and 89 ticks; forty filled
in a block died 5 to 6 ticks on, nudged by their neighbours' writes. A fan or plant losing its ground
(the tick's `canSurvive`) is left to sapling's support rules.

## Nether vines (`blocks/growth/kelp.rs`)

Weeping and twisting vines subscribe to the queued tick and not to the random tick, as kelp does, so they
share kelp's random-queue growth: a geometric wait drawn from the Java game's chance (0.1 a random tick,
`NetherVines`) at speed 3 on a queue that runs `randomTickSpeed` steps a tick; then the tip grows a cell
into air (down for weeping, up for twisting), a year older, to age 25.

## Chorus (`blocks/growth/chorus.rs`)

Java `ChorusFlowerBlock.randomTick`: with air over it and age under 5, it grows up (becoming a plant) when
on end stone or air, or on a stalk shorter than 2 (or of length l <= `nextInt(4)`, `nextInt(5)` on end
stone, counted up to 4 more) and the cell over has no neighbours and air over it; else under age 4 it tries
`nextInt(4)` (+1 on end stone) random sides with air there and under, each free of other neighbours, each
a flower one older; with none it dies at age 5. Sounds `chorusgrow` per new flower, `chorusdeath`.

## Amethyst (`blocks/growth/amethyst.rs`)

Java `BuddingAmethystBlock.randomTick`: one in five, a face of six at random; air or a water source there
gets a small bud facing that way (`minecraft:block_face`), a bud facing that way grows a stage, up to a
cluster. Water stays as layer 1.

## Sponge (`blocks/sponge.rs`)

Java `SpongeBlock`: a dry sponge written, or one beside a write, queues a tick this tick; it soaks up water
breadth first through water, 7 steps from it (BDS soaked a channel 7 long, bds-t3sponge.txt; Java's is 6),
at most 64 cells (water, flowing water, bubble columns,
waterlogged water; kelp and seagrass break), and turns wet. A wet sponge written in the Nether dries at
once, with event 3501 0.875 up its cell (drilled, below; dragonfly's `Sponge.UseOnBlock`, Java `WetSpongeBlock`).

The delay, read 2026-11-04: the place event (14660fbe0) soaks at once; `neighborChanged` (1466107c0, vtable
slot 74) queues a tick of delay 1, and the queued tick (146610180) soaks if dry or dries in the Nether.
`g3fan` (bds-g3fan.txt): water set beside a dry sponge, and a sponge set into water, were soaked in the same
game tick as the command's write, event 2021 sent with the water's UpdateBlock. Sapling's this-tick queue
already gives that; nothing changed.

## Lectern (`blocks/lectern.rs`)

A hit (StartBreak) on a lectern with a book drops the book over it and clears `book`, `hasBook`, `page`,
`totalPages`. Dragonfly's and PocketMine's `Lectern` take it off by a punch; the client's Take button was
not captured.

## Burning shots and TNT (`blocks/target.rs`, `creatures/flight.rs`)

A shot that burns (`Shot::burns > 0`: Flame arrows, small fireballs and dispensed fire charges) and is
stopped by TNT lights it (Java `TntBlock.onProjectileHit`). Fire lighting TNT, TNT in water breaking
nothing, blasts throwing entities and primed TNT saved with its column were already built (fire.md,
block-uses.md "TNT").

## Parity drill (2026-10-29, `bds_command_probe bkticks` and `bknether`)

Captures research/captures/bds-bkticks.txt, bds-bknether.txt against sapling-*.

- Same: chorus on end stone growing (`chorusgrow`), budding amethyst and vines growing under fast
  ticks; a sponge soaking the pool and breaking its seagrass and kelp; a lectern with a book broken in
  survival dropping the book, then the lectern.
- A sponge soaking is told by event 2021 (data 1302137) and `sponge.absorb`, both at the cell's
  corner (`blocks/sponge.rs`). Fixed.
- A wet sponge drying in the Nether: event 3501 at 0.875 up the cell, not `ParticleEvaporateWater`.
  Fixed.
- A small fireball (a dispensed fire charge) meeting TNT lights the cell before the face it met, and
  not the TNT (`blocks/target.rs`, Java `SmallFireball.onHitBlock`). Fixed.
- `/setblock` of a live coral fan or wall fan (bds-fixset.txt) puts a water source in its layer 1,
  which runs from the next liquid tick; a dead fan stays dry. Sapling's wrote none; fixed
  (`blocks/commanded.rs`, test `a_live_coral_fan_set_by_command_keeps_a_water_source_beside_itself`).
  BDS shows fan and water in one UpdateSubchunkBlocks, sapling in two UpdateBlocks. The coral block
  died about 45 ticks after it was set on BDS, 80 on sapling: one sample, Java's wait is 60 to 99.
- `/setblock` gives weeping, twisting and cave vines a random age (2 to 17 seen, bds-fixset.txt),
  and kelp under water (19, bds-bkticks.txt); kelp under glass kept 0. Sapling gave 0; fixed
  (`nextInt(25)`, same file, test `nether_and_cave_vines_set_by_command_take_a_random_age_unless_one_is_named`).
- Kelp and vines losing their hold, drilled 2026-11-03 (`bds_command_probe plantpop`, `plantpop2`,
  research/captures/bds-plantpop*.txt; tests in tests/integration/plants_drilled.rs). Fixed (`blocks/ground.rs`):
  kelp on glass goes at a neighbour's change, leaving its water, on stone it stays (magma, refused as Java
  `KelpBlock` does, was not drilled); twisting vines go
  without a top under them, weeping vines without a bottom above, unless on their own kind. Kelp set by
  command into air takes a water source beside itself, as a live coral fan does (`blocks/commanded.rs`).
- Fixed: water run in beside twisting, weeping or cave vines breaks them (`ParticleDestroy` as it enters
  layer 1, the vine's loot, the water then `liquid_depth=1` in layer 0); sapling's stood in it
  (`blocks/liquid.rs` `set`, bds-plantpop2.txt).
- Packet shape, not matched: BDS shows a commanded block written twice in a tick as one UpdateSubchunkBlocks
  listing each write (an aged vine twice; kelp or a coral fan with its water as both `water` and
  `flowing_water` in the extra layer); sapling sends an UpdateBlock a layer. Matching it needs a log of every
  write in the tick, with layer 1, which sapling's batches (`send/sections.rs`) do not keep.
- Undrillable here: arrows (`/summon arrow` is inert in sapling's drill).

## Parity drill (2026-11-04, `bds_command_probe t3odds`, `t3sponge`, `t3uses`)

- A dry coral block died exactly 45 ticks after it was set, all nine timed (bds-t3odds.txt; the one
  sample above was 45 too). Sapling drew Java's 60 to 99; fixed for coral blocks (`blocks/coral.rs`).
- Odds at `randomtickspeed` 40 for a minute (counts of cells shown, BDS vs sapling): chorus flowers
  243/215 and plants 191/168 from 7, weeping vines 91/78 and twisting 82/74 from 12 each, amethyst
  buds small 37/30, medium 12/19, large 4/2 from 9. Within one sample of each other; Java's odds kept.
- Sponge: BDS soaked a channel 7 long (above). Count (64) not reached.
- A punch on a lectern with a book drops the book and keeps the lectern on both (bds-t3uses.txt).
  "Sapling does not send BlockStartBreak to the actor" was wrong: it does (sapling-bkblocks.txt).
  What differed: BDS's survival punch that knocks the book out still starts the break, with
  `BlockStartBreak`, the punch particles, then `BlockBreakSpeed` of the same speed, in that order
  (bds-bkblocks.txt, bds-t3uses.txt); sapling stopped at the book and sent the particles first.
  Fixed 2026-11-04 (`request.rs` `punch`, `mining::start`); test in survival.rs (the cracks).

## Not drilled

- BDS plays `place` with the dead block's id when coral dies; sapling's `coral::die` writes it with no sound.
- A coral fan placed by hand into water: the `g3fan` place was refused on BDS (NotVisible).
- Burning arrows lighting TNT, drilled since (bds-u1fire.txt, bds-u1blaze.txt, 2026-11-04): a player's
  flame arrow primed TNT it struck on both servers (`SoundIgnite` at the block's middle, then the TNT);
  a plain one did not. A blaze's fireballs set fire only under `mobGriefing`, as sapling's now do
  (`blocks/target.rs` `shot_stopped`, test `blaze_griefing`). Sapling's arrow sticks 0.125 short of
  the face it hit (7.875 against BDS's 8.0, its box against BDS's ray).

## Coral fan by hand

`g3fan` (2026-11-04, bds-g3fan.txt with bds-e3coral.txt's four, nine places): a fan placed dry was
`dead_*_coral_fan` at once, its place sound the dead id. `coral_fan_direction` fit PocketMine's
`CoralFan::place`: from the placer's feet cell to the fan, `atan2(dz, dx)` strictly between 45 and 135
degrees (the fan south of the feet) gives 0, anything else 1; west, north, east and the four
diagonals all gave 1. Sapling: `coral::placed` from `place.rs`; test
`a_coral_fan_placed_dry_by_hand_is_dead_and_lies_by_where_it_is_from_the_placer`.
