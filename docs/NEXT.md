# sapling — what to do next

The work list, in order. What is built and how is in `DESIGN.md`; every gap it knows of is in its
"Open items" and in the `Not done` notes of each section. What the game itself does is in
`research/` (decompiled BDS, the Java jar's data); read that before guessing a rule.

## 1. Entities, then vanilla commands

Decided 2026-10-06, so that the drill probe (`examples/bds_mechanism_probe.rs`) runs on sapling as
it does on BDS and the two captures can be diffed:

1. **An entity API.** `EntityId` handles like `PlayerId`, plain-data answers: `spawn_entity(kind,
   pos)`, `entity(id)`, `entities()`, `move_entity`, `set_velocity`, `remove_entity`. Bodies are
   simulated on the server with BDS's gravity and drag and collide with blocks; no AI. One model for
   spawned mobs, the mobs a world folder stores (today statues, `mobs.rs`), lying items and falling
   blocks: built (`body.rs`, `sync.rs`, `entities.rs`, `creatures.rs`, `actors.rs`), stored mobs
   saved back where they are. Left: saving thrown items, orbs and falling blocks.
2. **Vanilla commands**: built (`sapling::vanilla`); the drill probe runs on sapling
   (`tools/sapling-drill.sh`, on testbox `tools/remote-drill.sh`). Left: what the BDS-against-sapling
   diff of the drill captures still shows (`research/captures/sapling-vs-bds-findings.md`).

## 2. Survival gaps still open

Each needs its rule looked up first (`bdsre` for BDS, the Java jar for data), then a shard test.

1. **Falling blocks on the wire as BDS sends them**: left the splash sound (being decompiled,
   `docs/research/splash-sound.md`); concrete powder hardening (the drill's landing matched BDS's).
2. **Pressure plates on BDS.** The drill pressed none, not even under a walking chicken; rerun with a
   hand-placed plate, a bot walking on with input, and `/testforblock` (findings file, "Drill bugs").

## 3. Things assumed, not verified

- Mirrored shaped recipes are accepted (`Recipe::is_laid_out`) from memory of the game; decompile
  `ShapedRecipe::matches` to confirm.
- A door's hinge follows the Java game's rule without its tie-break by where the click fell.
- `door.open` / `door.close` sound names are by the pattern of the others; a chest's own sounds were
  not found and are not sent.
- Piston orientation comes from other servers' code only.
- Block loot is the Java game's tables under Bedrock names (`sapling-data/data/block-loot.tsv`).
- Whether a vanilla client names a hopper's slots `Container` is the bot's assumption.
- A placed shulker box's `facing` is the face clicked (`shulker-ender.md`).
- An orb is taken within Java's pickup box once 7 ticks old (`orbs.rs`): one recorded pickup; BDS's
  touch rule was not decompiled.
- Wooden blocks burn by a list of names where BDS reads a material (`furnaces/fuel.rs`).
- Who fences, panes and walls join, and what gives a rail, wire or repeater support, are the Java
  game's lists and collision boxes; the dirt and sand tags sugar cane asks for are Java's
  (`docs/research/standing-and-shapes.md`, "Open checks").
- Stair corners and hanging-sign states by Java's rules; a placed skull's `SkullType` (-1 as BDS
  leaves it), the cauldron's `PotionId` defaults (`docs/research/block-entities.md`, "Open checks").
- The distance at which BDS closes an open container, whether a double chest asks both halves
  above, and the handler for `PlayerAction StopSleeping` (`docs/research/beds-and-containers.md`).
- Which block each liquid rule belongs to was read by where the block's name sits in BDS's code:
  sure for whole families (stairs, slabs, walls), not for single blocks (`waterlogging.md`).
- Lava burns what has no collision; BDS asks each block (`isLavaBlocking`, 23 overriding classes).
- A pressed plain plate stores `redstone_signal` 1 (decompile only).

## 4. A vanilla client session

No real client has played against any of the survival code: only acacia's bot has. One session with
the game itself, walking through break, place, doors, beds, chests, double chests, crafting by hand
and by the book, the stonecutter, furnaces and water, and noting every kick, desync and wrong
picture. Needs a person at the client.

## 5. Ground truth from the test BDS

Record the bot doing the same things against the BDS in `acacia/.testserver` and diff the packets
with sapling's (`tools/bds-drill.sh` runs a probe on a fresh world; `bds-blocks` is a copy kept for
the drills, port 19182): chest and furnace traffic was never recorded (`research/containers-crafting.md`
says where it is decompiled only), nor door and bed placement. One agent at a time uses that
server. A hopper's rate, shulker box drops, a furnace's result and fuel slots and experience orbs
were recorded on a testbox BDS by `cargo run -p sapling --example bds_container_probe` (2026-10-06).
