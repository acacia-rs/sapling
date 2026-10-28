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

## Not drilled

- An item frame popping when its block goes (inferred).
- A bed's burst: halves removed, blast at their middle, nothing with `respawnBlocksExplode` off
  (decompiled).
- Fire from a bed's or an anchor's blast (Java's rule; BDS's fire pass not read).
- Underwater TNT breaking nothing (pack JSON and parser read; sapling's existing rule).
