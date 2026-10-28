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

## Not drilled

- An item frame popping when its block goes (inferred).
