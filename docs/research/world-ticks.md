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

Live coral of the five kinds (block, plant, fan, wall fan) with no water in its cell or the six beside it
(water, flowing water or a bubble column, in either layer) queues itself `60 + nextInt(40)` ticks on and
turns into `dead_<name>` with its properties if still dry then (Java `CoralBlock`,
`BaseCoralPlantTypeBlock`). Queued on any write in or beside it; the wait is kept with the column.

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
breadth first through water, 6 steps from it, at most 64 cells (water, flowing water, bubble columns,
waterlogged water; kelp and seagrass break), and turns wet. A wet sponge written in the Nether dries at
once, with event 3501 0.875 up its cell (drilled, below; dragonfly's `Sponge.UseOnBlock`, Java `WetSpongeBlock`).

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
- Left: BDS's `/setblock` of a coral fan or wall fan writes water into its cell, which spreads and
  washes the coral plant away; sapling's writes none, so its fans die. The coral block died about
  45 ticks after it was set on BDS, 80 on sapling: one sample, Java's wait is 60 to 99.
- Left: BDS's `/setblock` gives kelp and nether vines a random age (19, 8, 7 seen); sapling's gives 0.
- Undrillable here: arrows (`/summon arrow` is inert in sapling's drill).

## Not drilled

- Coral's wait (Java's 60 to 99): one sample above.
- The nether vines' chance (Java's 0.1), chorus and amethyst odds: grown in kind only.
- Sponge reach and count (Java's 6 and 64) and the queued tick's delay.
- A punch on a lectern taking the book without breaking it.
- Burning arrows lighting TNT; whether BDS also asks `mobGriefing` for a mob's shot.
