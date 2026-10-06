# sapling — what to do next

The work list, in order. What is built and how is in `DESIGN.md`; every gap it knows of is in its
"Open items" and in the `Not done` notes of each section. What the game itself does is in
`research/` (decompiled BDS, the Java jar's data); read that before guessing a rule.

## 1. Survival gaps still open

Each needs its rule looked up first (`bdsre` for BDS, the Java jar for data), then a shard test.

1. **Lava and buckets.** Lava flows like water with its own delay and reach (`research/water.md` has
   the liquid tick; the lava numbers are not looked up). A bucket fills from a source and empties
   into a cell; `water_bucket`, `lava_bucket` and `bucket` are items that name no block
   (`blocks/items.rs`). Water meeting lava makes stone, cobblestone or obsidian.
2. **Falling blocks as entities.** Sand, gravel, anvils and snow layers are moved to where they land
   in one step today (`blocks/updates.rs`); the game spawns a falling-block entity and places the
   block when it lands.
3. **Pressure plates.** Go down under a player and up when it leaves; stone and wooden ones differ
   in what presses them. No redstone behind them yet, so only the state and the sound.
4. **Waterlogged blocks.** A block placed into water keeps the water in layer 1 where the game
   does; a broken one leaves it. Layer 1 is stored and sent already.

## 2. Things assumed, not verified

- Mirrored shaped recipes are accepted (`Recipe::is_laid_out`) from memory of the game; decompile
  `ShapedRecipe::matches` to confirm.
- A door's hinge follows the Java game's rule without its tie-break by where the click fell.
- `door.open` / `door.close` sound names are by the pattern of the others; a chest's own sounds were
  not found and are not sent.
- Piston orientation comes from other servers' code only.
- Block loot is the Java game's tables under Bedrock names (`sapling-data/data/block-loot.tsv`).
- Whether a vanilla client names a hopper's slots `Container` is the bot's assumption.
- A hopper moves an item every 8 ticks; BDS's code says 4 of its hopper ticks (`hoppers.md`).
- A placed shulker box's `facing` is the face clicked; an empty broken one drops without a tag
  (BDS may write `Items: []`); a Place into a furnace's result is refused though BDS's validation
  seems to allow it (`shulker-ender.md`, `furnaces.md`).
- Wooden blocks burn by a list of names where BDS reads a material (`furnaces/fuel.rs`).
- Who fences, panes and walls join, and what gives a rail, wire or repeater support, are the Java
  game's lists and collision boxes; the dirt and sand tags sugar cane asks for are Java's
  (`docs/research/standing-and-shapes.md`, "Open checks").
- Stair corners and hanging-sign states by Java's rules; a placed skull's `SkullType` (-1 as BDS
  leaves it), the cauldron's `PotionId` defaults (`docs/research/block-entities.md`, "Open checks").
- The distance at which BDS closes an open container, whether a double chest asks both halves
  above, and the handler for `PlayerAction StopSleeping` (`docs/research/beds-and-containers.md`).

## 3. A vanilla client session

No real client has played against any of the survival code: only acacia's bot has. One session with
the game itself, walking through break, place, doors, beds, chests, double chests, crafting by hand
and by the book, the stonecutter, furnaces and water, and noting every kick, desync and wrong
picture. Needs a person at the client.

## 4. Ground truth from the test BDS

Record the bot doing the same things against the BDS in `acacia/.testserver` and diff the packets
with sapling's: chest and furnace traffic was never recorded (`research/containers-crafting.md`
says where it is decompiled only), nor door and bed placement. Also time a hopper into a chest,
break a filled and an empty shulker box, and take results from a furnace (orb movement packets). One agent at a time uses that
server.
