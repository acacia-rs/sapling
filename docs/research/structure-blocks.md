# Structure blocks and animated loads (2026-10-28)

How BDS keeps and uses structure blocks, and what sapling copies (`crates/sapling-shard/src/blocks/structure_blocks.rs`,
`structure_blocks/act.rs`, `src/structures/animate.rs`, `src/structures/export.rs`). Captures:
`research/captures/bds-structblocks.txt`, `bds-structanim.txt` (drills `structblocks`, `structanim` in
`crates/sapling/examples/shared/structure_block_drill.rs`).

## Captured

- Entity of a block put where none was: `BlockEntityVersion` 0, `animationMode` 0, `animationSeconds` 0.0, `data` 1
  (save), `dataField` "", `ignoreEntities` 0, `includePlayers` 0, `integrity` 100.0, `isPowered` 0,
  `lastTouchedPlayerId` -1, `mirror` 0, `redstoneSaveMode` 0, `removeBlocks` 0, `rotation` 0, `seed` 0,
  `showBoundingBox` 1, `structureName` "", offset (0, -1, 0), size (5, 5, 5). Shown with the block; a tick later the
  block is shown again in the entity's mode: one put as `load`, `corner` or `data` turns `save`.
- StructureBlockUpdate: answered by UpdateBlock and BlockEntityData in the block's new mode. With the trigger, a save
  happens then; an instant load puts its blocks first (one UpdateSubchunkBlocks per subchunk) and shows the block a
  tick later; an animated load shows the block first.
- Names: a block keeps its save under the name as typed. `sb1` saved by a block was not found by
  `/structure load sb1` (`mystructure:sb1`); one saved as `mystructure:sb5` was.
- Power: a save block's rise saved (`sb4` queried afterwards).
- StructureTemplateDataExportRequest: a query answers `Query` with the template, or success false and no NBT for an
  unknown name; export from save (the box the request's settings take) and from load answer `Export`. The NBT has
  `format_version` 2 and `block_indices` as a list of int arrays, one per layer (only the first for a box without
  liquids).
- Mirror 2 of the editor flipped x within the box.
- A load's blocks, instant or one step of an animation: one UpdateSubchunkBlocks per sub-chunk listing the cells
  that are not air by y, x and z, then every cell in the template's order, changed or not; each block entity's
  BlockEntityData twice.
- Animated `/structure load`: `t` ticks after the load, `floor(t * N / T)` of N layers (layer_by_layer) or blocks
  (block_by_block) are in, T = seconds × 20: 3 layers over 2 s came at ≈14, 27, 40 ticks; over 0.5 s, 3 ticks
  apart; 27 blocks over 2 s, two every three ticks. Order: y, then x, then z of the template before it is turned.
  0 s puts everything at once.

## Drilled 2026-10-29

- `/structure load` animations (`bds-structanim.txt`): sapling loaded at once; `StructureLoad` now carries the
  animation (`StructureAnimation`, `load_structure_animated`) and matches, 26 updates either side.
- With entities (`bds-structanim2.txt`, again `bds-t3turn.txt` 2026-11-04: 3 layers a second apart from a
  second after the command): the armour stand comes with the last layer on both.
- Export of a template holding a liquid (`bds-t3turn.txt`, a coral fan in water): the second layer of
  `block_indices` is sent whole, -1 but at the liquid's cell, as `structures/export.rs` sends it.
- The editor's click (`bds-structclick.txt`): ContainerOpen `StructureEditor` alone; sapling sent the entity
  first, as for a command block; fixed. A structure block placed from the hand in any mode is `save` at once
  (sapling showed `data` first); fixed (`place.rs`, `structure_blocks::as_placed`).
- Saves by power and from the editor (`bds-structpower.txt`): all three queried back alike on sapling, but for
  the empty `entities` list, which BDS sends untyped (tag 0); fixed (`structures/export.rs`).
- Re-drilled `structclick`: the same. `structpower2` on sapling: the same three refusals.
- After a restart (`bds-structpower2.txt`): none of three saves by block (by power with `redstoneSaveMode` 0
  and 1, from the editor) could be queried or loaded; BDS logs "Structure 'pw0' must be prefixed by a
  namespace" for each bare name.
- Integrity (`bds_command_probe t3integ`, bds-t3integ.txt, 2026-11-04): `/structure load ... 50 abc` of an
  8x8 stone row kept 36 cells. BDS's draw (141f4ae40) keeps cell `i` of the template's order when the first
  `nextFloat` of an MT19937 seeded with the seed's Java string hash plus `i`, times 100, is below the
  integrity; sapling's own dice kept others. Fixed (`structures/place.rs`, cell for cell; test
  `half_an_eight_by_eight_row_by_seed_abc_is_bds_half`). A bare number as the seed (`50 1`, `50 12345`) is a
  syntax error at it on BDS; sapling loaded; fixed (`vanilla/structure.rs`).
- A structure block's numeric `seed` (bds-e3seed.txt, 2026-11-04: 0 twice, 1, 12345, 4294967289 at 50) seeds
  the same draw with its low 32 bits as they are: 0 picked the same cells twice and `n` is the cells of 0
  moved on by `n`. Sapling hashed the digits and drew 0 from the tick; fixed (`structures/place.rs`, test
  `a_structure_block_s_numeric_seed_is_taken_as_it_is`).
- No seed (or `""`) draws one per load (bds-q3integ2.txt, 2026-11-05: the actor's and the watcher's
  `execute as @a ... structure load ... 50` in one tick placed different cells; three loads 3 s apart
  differed too, bds-q3integ.txt). Sapling seeded from the tick, so one tick's loads matched; fixed
  (`structures/place.rs`, test `two_unseeded_half_loads_in_one_tick_place_different_cells`).

## Not copied or not drilled

- After a load (bds-e3turn.txt, 2026-11-04: 64 turning blocks at 90, 180, 270 and mirrored, written as
  sapling writes them, 756 cells alike) BDS lets the loaded blocks react to their neighbours: what lacks
  support goes with its item (redstone torches, cocoa, activator rails, hanging signs), glow lichen and
  resin lose unheld faces, fences and walls join only what is there, the first write of a fence already
  short of later cells. On BDS the breaking showed in the load's tick and the blocks went, with their
  items, at the next, with the lichen's lost faces. Sapling (2026-11-05, `blocks/updates/loaded.rs`, test
  `a_loaded_structure_s_blocks_react_to_what_is_around_them_as_on_bds`): fences and walls reshape to their
  neighbours, glow lichen and resin keep only faces with something solid behind (none: it goes), and a
  torch, activator rail or cocoa that `lacks_support` shows breaking and goes a tick later. BDS kept levers,
  plain and golden rails, comparators, petals, leaf litter and big dripleaves that `lacks_support` would
  refuse, so only those three pop. Re-drilled (sapling-e3turn.txt, last state of the 1108 loaded cells both
  sides sent): 31 differed before, 9 after, all BDS taking out what sapling keeps: four hanging signs, three
  small dripleaves and two ascending activator rails (rail_direction 2 and 5).
  A small dripleaf `/setblock` on stone broke a tick later on BDS (sapling kept it).
  Settled 2026-11-05 (twelfth5, `t5holds`, bds-t5holds.txt). Of the four loads BDS sent (its two others,
  90 mirrored on x and 0 mirrored on z, showed no cells), the last diff had 11 cells: four hanging signs,
  four small dripleaves, three rising activator rails. None was the load's own rule:
  - Dripleaves: BDS's structure held air there, the dripleaf having gone before the save. A `/setblock`
    of a lower half writes its upper over whatever is above (here stone), both in one
    UpdateSubchunkBlocks, the upper first; off its ground both show breaking at once and go a tick later
    in one UpdateSubchunkBlocks, leaving nothing (shears only). It stood on clay, moss and dirt under
    water; not on stone or dry dirt (`mayPlace` 1466059d0). An upper half set alone stayed, and went at a
    neighbour's change; taking the upper off a standing one took the lower a tick later. Sapling
    (`commanded::put_dripleaf`, ground.rs, updates/unheld.rs; its loot was its own item, now Java's
    `small_dripleaf`, shears only).
  - Rising rails: a rail needs the cell it rises to at its own height (`BaseRailBlock` 142cd5660: x±1 or
    z±1 with the rail's y); sapling asked the cell under that one. A loaded rail rising out of the
    structure meets air. Drilled: rising to stone it stayed, went with the stone; rising to air it went as
    set.
  - Hanging signs: `hanging` ones need a bottom to hang from above (a chain or fence holds one, a top
    slab not, nothing not), asked at a neighbour's change, not as set (a sign set in air stayed till its
    neighbour changed); a wall one (`hanging` 0) set in air stayed. Sapling: ground.rs, and the load's
    after-update pops them (`loaded.rs`).
  Re-drilled on sapling (sapling-e3turn.txt, 2026-11-05): the last state of all 756 cells both sides sent
  alike. Tests `a_rising_rail_needs_the_cell_it_rises_to_at_its_own_height`,
  `a_hanging_sign_goes_at_a_change_beside_it_without_a_bottom_above_to_hang_from`,
  `a_small_dripleaf_set_on_stone_comes_whole_and_goes_whole_and_its_lower_half_goes_with_the_upper`
  (block_holds.rs); sapling-t5holds.txt's sounds, particles and items match BDS's, block for block.

- Placement and the editor by creative operators only, as for command blocks (a survival click and place were
  refused on BDS too).
- A save from the editor goes to the world; one by power follows `redstoneSaveMode` (1: world, else memory).
- Data and corner modes do nothing on the server; export mode's file is the client's.
