# sapling vs BDS: mechanisms capture

Sources: `bds-mechanisms.txt` (ground truth) and `sapling-mechanisms.txt`, same probe
(`crates/sapling/examples/bds_mechanism_probe.rs`), compared drill by drill. Ticks = ms / 50; watcher times are the
reliable ones (bds-mechanisms-findings.md has the format). Sapling y = BDS y + 124. Not repeated here: the known
items in the brief (UpdateSubchunkBlocks batching, UpdateBlockSynced, MOVING/Health metadata, plates never pressed,
wet-stairs break logged Rejected).

Fixed since bds-mechanisms-findings.md, now matching BDS: falling-block spawn state (+0.46, vy -0.0392), wire offset
0.49, motion only at |dv| > 0.2 and on landing, `ticks` 0, block at L+1 and removal ~3 ticks later, refused landing
without on_ground packets with particles at the wire position, anvil sound at the cell corner on L, hand-placed sand
falling in its tick, random fizz pitch, no implicit layer-1 packet, place sound from the default state, snow
particles at the corner.

## Commands (setblock / fill / tp / summon / kill)

1. **/fill and /setblock send and count cells that did not change.** `send::set` (send.rs:129-146) writes and
   broadcasts with no "same block" check (sapling-world world.rs:232 always writes), and `updates::settle` runs per
   cell. The probe's `fill ... air` over air (109 x 7 x 17 = 12971 cells) became ~13k UpdateBlocks: still arriving
   at 11 s (~28 a tick, x -32..-16, sapling lines 6-1398), and the first two drills found their cells "not loaded"
   because chunks queued behind them. BDS sends nothing for an unchanged cell. Fix: in `send::set` return false when
   block and layer 1 are already what would be written; `vanilla/blocks.rs:60` then counts only real changes
   (BDS: setblock of the same block is a failure, "The block couldn't be placed").
2. **A liquid placed by /setblock schedules its own tick; on BDS only its neighbours are woken.**
   `update_block` -> `updates::settle` (updates.rs:92) -> `liquid::disturb` includes the cell itself
   (liquid.rs:142, `.chain([at])`). Evidence, two drills:
   - bucket drill: `setblock water ["liquid_depth"=3]` with no source stayed depth 3 on BDS for the whole step
     (before and after the click); on sapling it had dried to air before the click.
   - lava column: BDS lava (placed in setup) first fell 30 ticks after the neighbour `/setblock stone`
     (221697 -> 223202); sapling 15 ticks after (200925 -> 201674): its own tick from the setup was already running.
   Fix: settle's disturb for the changed cell should queue only neighbours (BDS `neighborChanged`); `liquid::set`,
   `pour` and `take` keep queueing the cell themselves.
3. **No command feedback Text.** BDS sends the tp'd player `Text commands.tp.successVictim` and every other operator a
   grey `§7§o[MechActor: commands.tp.success.coordinates]` (also `commands.summon.success`, `commands.kill.successful`,
   `commands.setblock.success`; 23 Text packets). Sapling sends no Text at all; output goes only to the sender as
   CommandOutput (commands.rs:159-170). Fix: an admin-broadcast of the output to other operators (gamerule
   sendcommandfeedback) and the victim line in `/tp` (vanilla/players.rs:46-58). Its English output strings vs BDS's
   translation keys were not captured (CommandOutput is not in SHOWN).
4. **/summon:** BDS follows AddEntity with `LevelSoundEvent "spawn"` (entity_type set, near the mob's centre: chicken y+0.68, pig y+0.77, armor stand y+1.68),
   a full-metadata SetEntityData and a flags SetEntityData; AddEntity velocity y -1e-5; the chicken and pig then
   moved/turned. Sapling: AddEntity only, mob never moves (creatures.rs:4 TODO, no AI). Add the spawn sound in
   `spawn_entity` (creatures.rs); the rest is the known AI gap.
5. **/kill:** BDS drops loot at once (chicken: item 275; pig: porkchop 262 x3, both with the item SetEntityData
   sequence) and removes the mob ~20 ticks later (death); armor stand: removed at once, no drop. Sapling removes
   every mob at once with no loot (players.rs:143-152). Low until mobs have damage/death.

## Falling blocks

6. **The whole flight is one tick behind BDS.** BDS sends AddEntity (state 1 tick in) *and* the 2-tick delta
   (y -0.0792) in the spawn batch (52178 both); sapling sends the first delta a tick later (34422 -> 34474, 47186
   -> 47236, 72906 -> 72955). Same delta sequence, so on_ground, the block (L+1) and removal all come one tick later
   relative to the support break (anvil: 12 vs 13 ticks). Cause: falling.rs:101-106 moves existing entities
   (`flight::fly`) before starting new falls, and `flight::start` (flight.rs:45-53) steps once. Fix: step a second
   time in `start` and send that delta after the announcement (needs item 7's ordering fix first).
7. **Entities stepped in their spawn tick send a MoveEntityDelta before their AddEntity.** Refused sand on the slab:
   `22596 MoveEntityDelta rid 4` -> `AddItemEntity rid 4` -> second delta. `drops::fly` (handlers.rs:246,
   drops.rs:121-136) steps and sends deltas straight to `Outgoing`, while the announcement waits for `mobs::update`
   (shard.rs:97); `reannounce` also makes AddItemEntity carry the post-step state (vy 0.1568). BDS: AddItemEntity
   with the spawn state (vy 0.2), then SetEntityData (full metadata) + delta + motion the next tick. Affects every
   item spilled before `drops::fly` in a tick (block breaks, buckets, falling blocks). Fix: don't step an entity in
   the tick it spawned, or push its announcement to current viewers at spawn.
8. **Refused landing: item spawned at a random point of the cell, inside the slab.** `break_up` -> `loot::spill`
   -> `drops::spill` (flight.rs:69-75, drops.rs:94-100) picks x/y/z in 0.15..0.85 of the cell; sapling's sand item
   started at bottom ~64.19 under the slab top 64.5 and, with nothing pushing it out (body.rs:4 TODO), fell through
   the slab to the dirt. BDS: item at the entity's exact x/z, item bottom = entity wire y
   (AddItemEntity `(-31.5, -58.885, 2.5)` for wire -59.01), velocity (+-0.1, 0.2, +-0.1). Fix: lay the item at
   `[x, bottom + 0.49, z]` with spill's motion.
9. Refused landing timing: BDS ParticleDestroy on L, AddItemEntity + RemoveEntity on L+1; sapling all on L
   (flight.rs:100-105). Low.
10. Anvil: BDS `SoundAnvilFall` precedes the on_ground delta in the batch; sapling appends it after delta + motion
    (flight.rs:111-117). Landing SetEntityMotion y is `-0.0` on sapling (body.rs:73, `*= bounce 0.0`), `0.0` on
    BDS. Both cosmetic.
11. Concrete powder: still no `splash` LevelSoundEvent (BDS on the landing tick, before the on_ground delta,
    `extra_data 1653280`, entity_type falling_block, at the entity). Landing y, block and no hardening match.
12. Snow, anvil, stack (one start per tick), hand-placed sand (falls in its tick): match apart from item 6.
13. Untested on sapling: "sand onto dirt" and "gravel onto a torch" (cells not loaded, item 1).

## Buckets

14. **Item 2 again:** the "empty bucket on flowing water" drill clicked air on sapling; nothing to compare.
15. Water players walk into: BDS sends `LevelSoundEvent "splash"` at the player (y + 0.9, extra_data ~263904) when
    flowing water reaches it (bucket and lava drills); sapling never does. Low, client-side physics otherwise.
16. Refusals (adventure lava bucket, lava aimed into the ground): sapling sends InventoryContent (buckets.rs:53-55
    runs on failure too); BDS sent no inventory packets for the adventure refusal. Resync positions/layers differ as
    already noted (buckets.rs:51 vs BDS's 6 layer-0 cells). Low.
17. BDS delivers the bucket's block change + sound one batch after the resync/InventoryContent (26-55 ms later);
    sapling sends block + sound first, then InventoryContent twice (each with all four windows). Equivalent.
18. Water into grass: BDS dropped seeds (item 291), sapling none; seeds are a chance roll (buckets.rs:124-127 does
    roll), one sample proves nothing.
19. Sounds at the feet with -1, item swaps (363/365/366), snow particles at the corner, flow from the snow and grass
    sources (including the infinite source between them): match.

## Lava

20. Fizz timing: BDS sends `SoundFizz` a tick before the hardened block (214894 fizz -> 214942 cobblestone;
    226144 -> 226198 stone; 229142 -> 229191); sapling sends block then fizz in one tick (harden.rs:45-51).
    Probably BDS flushing block changes a tick late, not a different rule. Low.
21. Sapling writes flowing lava into a cell and hardens it in the same tick: two UpdateBlocks for one cell
    (207675 `(18,64,-4)` lava then cobblestone; also `(19,64,-3)`). BDS never showed that. Low (wire noise).
22. Drop-off, 30-tick steps, lava -> cobblestone/stone, wall-break timings (+30 ticks), water 5 ticks: match.

## Waterlogging

23. **Water leaves a waterlogged block through every side on sapling; BDS keeps it out of the stairs' full face.**
    Water bucket on east-facing `oak_stairs` (weirdo_direction=0) beside the emptied pit 2 east: BDS spread
    north and south only ((-2,1), (-2,3) depth 1, +6 ticks), never east, though east is the short way to the
    drop; sapling spread east only ((-1,64,2) depth 1, +5 ticks, then down into the pit). flow.rs `slope` /
    `spread_to` (flow.rs:61-104) check only the target cell, not the face of the waterlogged source block.
    Fix: when the flowing cell holds its water in layer 1, skip sides whose face of that block is full (stairs
    back, etc.); confirm the exact rule in the decompile (`LiquidBlock::_canSpreadTo` / `canContainLiquid`).
    Medium confidence (one drill), but the E-vs-N/S split is unambiguous. Also the first neighbour: BDS +6 ticks,
    sapling +5.
24. Breaking a block: BDS sends ParticleDestroy before the UpdateBlock (every break in both captures); sapling the
    other way round (blocks.rs:261-264). Cosmetic.
25. Stairs into a source / stone into a source / stairs into flowing water: same packets and sounds (layer 0 only,
    place sound -122484237); BDS staggers a receding stream's cells over neighbouring ticks, sapling steps all on
    exact 5-tick boundaries. Equivalent.

## Drill and bot issues (not sapling bugs)

- Bots ask for chunk radius 2 (acacia-client client.rs:90 default; probe.rs:45 sets none) and sapling grants 2
  (join.rs:19, 113). The watcher (just west of 0) never had chunk (1,-1) or x >= 32: it missed the whole lava column,
  the waterlogging placements and breaks, and the bots dropped each other (RemoveEntity 1/2 at 195947) when water
  pushed the actor to z < 0. BDS's watcher saw all of these, so BDS grants more than 2 (its ChunkRadiusUpdate was
  not captured; worth checking whether BDS has a minimum radius sapling should copy). Fix the probe:
  `.chunk_radius(8)` in `shared/probe.rs::connect`.
- `stand` gives up silently after 100 ticks; make it fail so "not loaded" drills stop there.
- The moved bucket clicks ("lava aimed into the ground", "creative water bucket") now succeed on sapling but were
  out of reach on BDS: rerun the probe on BDS.
- Punch-to-break takes ~7.6 s for stone on both servers: the bot's break, not the server.
- Pressure plates, armor stand/pig stillness and the flooded lava column: as in bds-mechanisms-findings.md.
