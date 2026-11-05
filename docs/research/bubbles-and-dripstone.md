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
survival player the tip lands on is hurt.

### Drilled 2026-11-05, group I (`bds_command_probe i2drip`, `i2grow`; thirteenth2i_drill.rs)

- **A stalagmite losing its ground** (bds-i2drip.txt, a frustum and tip on stone, the stone set to
  air): the frustum shown breaking in the same tick, gone with its item the next, the tip shown
  breaking then and gone the tick after. `canSurvive` (14660c830): a standing piece needs a standing
  piece or a whole top under it. Sapling left it standing; fixed as a ground rule
  (`blocks/ground.rs`, which updates/loosened.rs breaks a tick on), re-drilled the same to a tick;
  test `a_stalagmite_whose_ground_goes_breaks_piece_by_piece_a_tick_apart`.
- **Falling on a stalagmite** (`SpeleothemBlock` fall-on 14660bc00, registered with a
  `BlockEntityFallOnEvent` lambda): on a standing tip the fall counts 2 blocks more, twice over,
  as `Stalagmite` (HurtAnimation 29, no landing position). BDS cost a survival player 8 dropped
  from y -20 onto a tip in -26 (stone 3). Sapling counted a plain fall; fixed (`creatures/health/falls.rs`
  `Ground::Spike`, `creatures/heat.rs`), test `a_stalagmite_tip_counts_two_blocks_more_twice_over`.
  Sapling's drill could not show it: the drill bot fell through the tip (BDS put it on top).
- **Growth** (14660e6b0, the block's constructor gives 7, true, 0.011377778): a root's random tick
  draws the growth first; then a coin: stalagmite (14660eb20) or the stalactite's own tip
  (14660e9e0). Either needs the root's tip within 7, unwaterlogged, a dripstone block over the root
  with a water source over that, and air or a meeting tip under the tip. A stalagmite: the first of
  the ten cells from the tip down holding a standing tip with room grows it up, or the first over a
  standing piece or a whole top gets a new tip (no Java `canDripThrough` stop). A tip met by another
  merges both (`merge`). The neighbours' `neighborChanged` (14660ceb0) sets each piece's thickness
  (14660c4b0, Java's `calculateDripstoneThickness`). bds-i2grow.txt at `randomtickspeed` 4096: the
  stalactite over air grew to -25, a stalagmite rose to meet it and the two merged; the one over a
  stalagmite's tip two below merged with it; the one under stone never grew. Sapling grew nothing;
  fixed (`blocks/drips/growth.rs`, thickness in `blocks/shapes.rs`), re-drilled: the same shapes
  and merges, in one packet each as BDS's, test `a_stalactite_under_a_dripstone_block_under_water_grows`.
  Sapling's growths came some four times slower than BDS's at the same rule: it ticked a cell at most
  once a tick, where BDS's picks repeat past a rule of about 1600, and it fell behind at 4096; fixed
  (growth.md "The random tick's pace", `l2pace`). Left: BDS drew the stalactite's own tip 5 times of 7,
  sapling 2 (too few to tell the coin).
