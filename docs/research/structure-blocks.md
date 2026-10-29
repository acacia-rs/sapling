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
- With entities (`bds-structanim2.txt`): layer by layer over 2 s, the armour stand comes with the last layer on
  both.
- The editor's click (`bds-structclick.txt`): ContainerOpen `StructureEditor` alone; sapling sent the entity
  first, as for a command block; fixed. A structure block placed from the hand in any mode is `save` at once
  (sapling showed `data` first); fixed (`place.rs`, `structure_blocks::as_placed`).
- Saves by power and from the editor (`bds-structpower.txt`): all three queried back alike on sapling, but for
  the empty `entities` list, which BDS sends untyped (tag 0); fixed (`structures/export.rs`).
- Re-drilled `structclick`: the same. `structpower2` on sapling: the same three refusals.
- After a restart (`bds-structpower2.txt`): none of three saves by block (by power with `redstoneSaveMode` 0
  and 1, from the editor) could be queried or loaded; BDS logs "Structure 'pw0' must be prefixed by a
  namespace" for each bare name.

## Not copied or not drilled

- Placement and the editor by creative operators only, as for command blocks (a survival click and place were
  refused on BDS too).
- A save from the editor goes to the world; one by power follows `redstoneSaveMode` (1: world, else memory).
- When an animated load brings in its entities (with its last step here); the export form of a liquid layer.
- Data and corner modes do nothing on the server; export mode's file is the client's.
