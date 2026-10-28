# Bubble columns and pointed dripstone

Measured 2026-10-27 with `bds_command_probe bubbles` and `dripstone`
(`crates/sapling/examples/shared/nature_drill.rs`; `research/captures/bds-bubbles.txt`,
`bds-dripstone.txt`), and read from the bds-molang build with bdsre. [D] decompiled, [I] inferred.

## Bubble columns

What BDS did:

- Soul sand set under a tube of water sources: the soul sand and a `bubble_column` (`drag_down`
  0) in the cell over it in one UpdateSubchunkBlocks, the water in layer 1 (`extra`). The column then
  climbed a cell a tick to the top of the water.
- Magma set in place of it: the same, `drag_down` 1, a cell a tick.
- Stone set in place of the magma: the cells turned back to water sources from the bottom up, one
  every six ticks.
- A cell of the column set to air: water there at once (the water in layer 1), a column again six
  ticks on, and the cell over it water and then a column again.
- A pig in the rising column was thrown up out of its top; one in the dragging column was held
  down and drowned (`HurtAnimation` 22, then death). No sound from the column itself: the bubble
  sounds are the client's (`animateTick`, 141a2eb10).

The rules [D]: `BubbleColumnBlock` (vtable 14a764e40) ticks a cell a tick after it is placed and
six after a neighbour changes; a tick with no soul sand, magma or column under it removes it, else
the water source over it becomes a column too, so it climbs a cell a tick. Soul sand and magma start
one over themselves. The push is `EntityInsideSystem` (1446abb90, 14469a100), the speed up alone, for
any actor: inside a rising column `min(vy + 0.06, 0.7)`, in its top cell with air over it
`min(vy + 0.1, 1.8)`; dragging, `max(vy - 0.03, -0.3)` and `max(vy - 0.03, -0.9)`. It also clears
the fall distance.

Sapling (`blocks/bubbles.rs`, `body/bubbles.rs`): any write queues the cell and the one over it to
become what the block under asks for, a tick on to form and six to turn back, so a column climbs
and falls back as BDS's did; the first cell comes a tick after the soul sand, where BDS showed both
together. Bodies the server moves (mobs, items, falling blocks) are pushed by the rule above;
players move themselves. A column's queued cell is kept with its column as water's tick.

## Pointed dripstone

What BDS did, a stalactite three long (base, frustum, tip) over a pig:

- Its stone set to air: the next tick the base and frustum went to air in one
  UpdateSubchunkBlocks, the tip in an UpdateBlockSynced, and three falling blocks were added, tip
  first.
- Each piece as it landed: the tip hurt the pig (`HurtAnimation` 28, stalactite; the pig lived),
  then `ParticleDestroyBlockNoSound` and `PointedDripstoneLand` at the cell, before the landing move;
  the next tick the piece put and taken away in one UpdateSubchunkBlocks and its item let go. The
  falling blocks went four ticks after landing.
- A stalactite under stone under a water source, a cauldron four under its tip,
  `randomtickspeed` 200: `ParticleDripstoneDrip` (data 28) at the tip now and then, and each fill
  some 54 ticks after a drip: `CauldronFillWater` at the water's new surface (data -1087090343),
  `fill_level` up by 2, `cauldron_drip.water.pointed_dripstone` at the cauldron. Lava (data 29) filled
  its cauldron to 6 at once with LevelEvent 3512 and `cauldron_drip.lava.pointed_dripstone`.

The rules [D]: a hanging piece that loses its hold queues every piece from the tip up a tick on;
`startFalling` (14660e100) has only the tip hurt, `min(1 + pieces over it, 6)` for each block fallen
past the first (`ceil(distance - 1)`), at most 40 (`FallingBlockActor::causeFallDamageToActor`
145985ca0). The random tick (1465fe4b0) of a root under a source over its block drips with odds
0.42 for water and 0.07 for lava (the Java game's are 0.17578125 and 0.05859375) and queues the
cauldron within ten under the tip, past air, `50 + the drop` ticks on; a cauldron takes water while
not full and lava only empty.

Sapling (`blocks/falling/dripstone.rs`, `blocks/drips.rs`) does all of the above; a mob or a
survival player the tip lands on is hurt. Left: a stalagmite that loses its ground breaks only
where updates.rs breaks things, a player falling on one is not hurt more, and dripstone does not
grow.
