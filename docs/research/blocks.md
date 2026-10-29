# Block leftovers: held stacks on break, and the G7 list

What sapling does for the blocks the parity round's "blocks" workstream covered, and where each rule
came from. BDS reads are from `acacia/.testserver/bds-blocks/bedrock_server.exe` with bdsre (`dis`;
`c` needs the Windows r2 and does not run on this machine). Tags: **BDSRE** read in the exe; **Java**
the Java game's rule; **df** dragonfly; **inf** inferred.

## Held stacks leave whole

A block whose entity holds stacks leaves each with its whole stack, tag included (a written book's
pages, an item's `ench`): lectern (`book`), item frame and glow frame (`Item`), decorated pot
(`item`), flower pot (its plant, which has no tag), campfire (`Item1`..`Item4`), jukebox
(`RecordItem`), chiseled bookshelf (`Items`). The same path serves a break by hand, a pop (lost
support, a piston, `ctx.break_block`), a blast and a shot pot (`blocks/entities.rs` `loot_tags`,
`blocks/loot.rs` `Tags`). df `block/lectern.go:46`, `block/item_frame.go:88` drop the stack itself.

- Tags go, in order, on the spilled stacks of their item; an untagged held stack keeps its place
  (`None`), so a shelf with a plain and a written book and quill tags the right one.
- An item frame or glow frame hangs by its `facing_direction`, as a ladder does, and pops when the
  block behind it goes (`blocks/updates.rs` `HANGS`). **inf** (Bedrock frames are blocks that need
  their face; not read in the exe).

## Beds outside the Overworld

`BedBlock::use` (142cd8100, **BDSRE**, `dis`): before anything else it asks the dimension whether
beds work there (vt+0xd0); where they do it goes on to `BedBlock::use_sleep` (142cd8730). Elsewhere
(Nether, End):

1. Game rule 33, `respawnBlocksExplode` (`cmp byte [rules + 33 * 0x118 + 4]`), off: nothing happens,
   the click is used up.
2. Rule 40, `allowDestructiveObjects` (Education, on by default), off: the bed is only removed.
   sapling has no Education rules and skips this.
3. Otherwise the clicked bed's head goes to air (setBlock flags 3), then its other half (142cd9bf0
   finds it) if it is still a bed, and `Level::explode` (vt+0x3c8) runs with no source at the middle
   of both halves' centres (`((h + 0.5) + (f + 0.5)) * 0.5`), power 5, `fire` 1, `breaksBlocks` 1,
   `maxResistance` `FLT_MAX`, `allowUnderwater` 0.

sapling: `blocks/bed_burst.rs`, called first thing in `sleep::lie_down`. Fire: `creatures/blast.rs`
`explode_burning` lights a third of the cells the rays reached (broken or free) that stand over a
solid block, the Java game's `Explosion.finalizeExplosion` rule (**Java**); BDS's own fire pass was
not read. The respawn anchor's burst goes through it too (Java's anchor blast has fire).

## Underwater TNT

The vanilla `minecraft:tnt` entity's `minecraft:explode` (behaviour pack `entities/tnt.json`) sets
`power` 4, `causes_fire` false and no `allow_underwater`; the component's parser (141bdbe30) reads
`allow_underwater` beside `breaks_blocks`, so it is false for TNT. With it false and the blast's
middle in water, BDS's explosion breaks no blocks (the bed passes the same 0 explicitly, above).
sapling already did this (`creatures/blast/rays.rs` `broken`: none when the middle is wet); nothing
changed. The Education `allow_underwater_bit` TNT state is not read by sapling.

## Infested blocks

A virtual of the infested blocks' vtables (142fd3ec0, **BDSRE**) returns at once when the break's
tool has silk touch (144bc8ef0), else tail-calls 142fd3cf0, which (after a check through the
region's vt+0x1c8) spawns `ActorType` 0x40b27 (silverfish) by `Spawner::spawnMob` at the cell's
integer coordinates, `surface` 1. Found by the `imm 40b27` immediate. sapling (`blocks/infested.rs`):
any `minecraft:infested_*` broken by a survival hand without silk touch (`after_break.rs`), or broken
and left to drop in a blast (`blown.rs`), spawns a silverfish at the cell's corner, unless
`doTileDrops` is off (the Java game's `InfestedBlock.spawnAfterBreak` gate; the vt+0x1c8 check may be
it, **inf**).

## Item frames: maps and comparators

- A frame holding a `minecraft:filled_map` has `item_frame_map_bit` 1, else 0; set whenever its item
  goes in, turns or comes out (`blocks/frames.rs` `show_map`). The client draws the map from the map
  data it asks for as for a held map. **inf** (the state's name; BDS not read).
- A comparator reads a frame the Java game's way (`ComparatorBlock.getInputSignal`, `getItemFrame`,
  `ItemFrame.getAnalogOutput`, **Java**): only past a solid block behind it, a frame on that block's
  back facing away from the comparator, 1 + its turn in eighths (1 to 8), 0 empty, whichever is
  stronger of it and the block there. A frame right behind the comparator reads nothing, which is
  what BDS showed for a flat frame (research/captures/bds-holders.txt, docs/research/redstone.md).
  sapling: `blocks/redstone/reads.rs` `read_past`.

## Lodestones

BDS keeps lodestone targets in its `PositionTrackingDB`: LevelDB key `PosTrackDB-` + `0x%08x` of the
handle (key built at 14478b480, **BDSRE**), and the last handle under `PositionTrackDB-LastId`
(1447a7540). The record's fields (`dim`, `id`, `pos`, `status`, `version` 1) and the compass's
`trackingHandle` tag (string found at 141a61530 and others) are the community's description of the
format, not read in the exe. sapling: `sapling-leveldb` `trackers.rs` (`TrackerRecord`), kept in
`Level.trackers`/`last_tracker` of the realm's overworld; `blocks/lodestone.rs`: a compass or
lodestone compass clicked on a lodestone is swapped for a lodestone compass tagged with the handle of
that lodestone's target (made at the first binding), heard as
`lodestone_compass.link_compass_to_lodestone`. A `PositionTrackingDBClientRequest` is answered with
a `PositionTrackingDBServerBroadcast`: Update with the record, Destroy (status 1) once the loaded
cell holds no lodestone, NotFound for an unknown handle.

## Big dripleaf

The Java game's `BigDripleafBlock` (**Java**; BDS has a queued-tick and a redstone subscriber for it,
14a7d8990/14a7d89c0, not read): a body whose feet are 0.6875 or more up a still, unpowered leaf head
makes it `unstable`; 10 ticks on `partial_tilt`, 10 on `full_tilt`, 100 on `none`; a shot that meets
it tilts it fully; power sets it back to `none` at its next step. Sounds `tilt_down.big_dripleaf`
on partial and full, `tilt_up.big_dripleaf` going back (names from BDS's sound table, 14346a2b0).
sapling: `blocks/dripleaf.rs`, `Job::Dripleaf`. Whether a fully tilted leaf lets bodies fall
through is the collision data's, not checked.

## Brushing

Suspicious sand and gravel: see structure-block-entities.md "Brushing".

## Parity drill (2026-10-29, `bds_command_probe bkblocks` and `bknether`)

Captures research/captures/bds-bkblocks.txt, bds-bknether.txt against sapling-*.

- Same: a frame popping when its stone goes; a framed map's `item_frame_map_bit`; underwater TNT
  breaking nothing; an infested block's silverfish heard at the cell's corner, from a survival hand and
  from a blast, and none with `doTileDrops` off (sapling's drill world is peaceful, so its silverfish
  went at once); a charged anchor's blast lighting fire (cells differ, random).
- A bed's burst is at the head's middle, not between the halves (`blocks/bed_burst.rs`; the foot then
  pops). Fixed. Which middle a click on the foot gives was not drilled.
- With `respawnBlocksExplode` off BDS did nothing; sapling's bot could not reach the second bed.
- TNT summoned under water makes no splash (`splash.rs`: the first check only notes the water). Fixed.
- A big dripleaf on stone breaks when it would tilt (`blocks/dripleaf.rs`). Fixed. On dirt BDS tilts
  as below: `unstable`, 10 ticks `partial_tilt`, 10 `full_tilt`, 100 back to `none`, power setting it
  back. Same, but its tilt sounds are at the cell's corner, not its middle. Fixed. Left: with the
  player still on it, BDS's leaf tilted again as soon as it came back up; sapling's did not (its
  player had dropped into the leaf's cell while it was tilted).
- A wet sponge set in the Nether: event 3501 0.875 up the cell, then the dry sponge
  (`blocks/sponge.rs`). Fixed.
- Left: BDS's `/setblock` of a bed writes both halves, sapling's only the one named (commands).
- Left: lodestones (not in the scenes).

## Not drilled

- A bed's burst clicked at its foot; fire from a bed's blast cell by cell (BDS's fire pass not read).
- A comparator reading a wall frame past a block, by turn.
- Lodestone: the record's fields, the broadcast's actions, a compass in creative being swapped,
  status 1 for a gone lodestone, and whether BDS tells clients unasked when one breaks.
- Big dripleaf: the 0.6875 height and the shot (arrows are inert in sapling's drill).
