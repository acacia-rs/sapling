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
5. **Shulker boxes and ender chests.** A shulker box keeps its items in the item when broken; an
   ender chest is one container per player, kept with the player's record.
6. **Hoppers move items.** They open as containers of five and do nothing else.
7. **Furnace leftovers.** Experience on taking the result, the crackling sound, a result slot that
   refuses items, the rarer wooden fuels, and a furnace that was burning when its world was saved
   (it stands still until someone opens it). Burn times are a table from the wiki in
   `furnaces/fuel.rs`; read them from BDS.
8. **Items that name no block, the rest.** Signs, skulls, banners, flower pots, cauldrons, brewing
   stands and campfires need their block entities; crops, sugar cane and redstone need their rules
   for where they may stand. A broken bed drops a white one: read the colour back from its entity.
9. **Sleeping in a bed**, rails, fences and stairs reconnecting, a reach check while a container is
   open, a chest blocked from above.

## 2. Things assumed, not verified

- Mirrored shaped recipes are accepted (`Recipe::is_laid_out`) from memory of the game; decompile
  `ShapedRecipe::matches` to confirm.
- A door's hinge follows the Java game's rule without its tie-break by where the click fell.
- `door.open` / `door.close` sound names are by the pattern of the others; a chest's own sounds were
  not found and are not sent.
- Piston orientation comes from other servers' code only.
- Block loot is the Java game's tables under Bedrock names (`sapling-data/data/block-loot.tsv`).
- Whether a vanilla client names a hopper's slots `Container` is the bot's assumption.

## 3. A vanilla client session

No real client has played against any of the survival code: only acacia's bot has. One session with
the game itself, walking through break, place, doors, beds, chests, double chests, crafting by hand
and by the book, the stonecutter, furnaces and water, and noting every kick, desync and wrong
picture. Needs a person at the client.

## 4. Ground truth from the test BDS

Record the bot doing the same things against the BDS in `acacia/.testserver` and diff the packets
with sapling's: chest and furnace traffic was never recorded (`research/containers-crafting.md`
says where it is decompiled only), nor door and bed placement. One agent at a time uses that
server.
