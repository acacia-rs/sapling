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

1. **Water and lava for entities**: items float and burn, orbs bob and are thrown out of lava,
   mobs sink, as on BDS (`body/liquid.rs`, `docs/research/water-movement.md` 7 and 8); concrete
   powder sets (`bds_command_probe concrete`). Mobs have health; lava, fire, falls, a
   block their head is in and water without breath hurt and kill them, cooked if on fire
   (`creatures/harm.rs`, `creatures/health.rs`, `docs/research/mob-damage.md`, `bds_command_probe
   burn` and `harm`). A stored mob's health and fire are
   saved with it. Water puts a fire out with BDS's fizz (`douse`). Left: attacks, and what the
   doc lists as not drilled.
2. **Redstone**: plates, buttons, levers, blocks of redstone and torches power dust, lamps, doors,
   trapdoors, gates and hopper locks (`blocks/redstone.rs`, `docs/research/redstone.md`; plugins
   read `ctx.power_at`), checked against `bds_command_probe redstone`. Circuits a world folder
   holds are found as their columns load. Repeaters pass, delay
   and hold (`bds_command_probe repeaters`). Comparators
   compare, subtract and read containers (`bds_command_probe comparators`). Observers
   pulse (`bds_command_probe observers`). Pistons push, break and
   draw back (`blocks/pistons.rs`, `bds_command_probe pistons`). Left: the circuits of an arena's
   template, what a comparator reads besides slots, slime and honey on pistons and what stands in
   a piston's way, and whether dust lags a lever, button or plate worked
   by hand as it lags an observer (docs/research/redstone.md, "Observers").

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
- Which items lava does not burn: netherite's and ancient debris, by name (`drops.rs` `fireproof`);
  BDS reads a flag of each item.

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
