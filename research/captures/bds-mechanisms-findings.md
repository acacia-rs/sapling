# BDS mechanisms capture: findings vs sapling

Source: `bds-mechanisms.txt` (probe `crates/sapling/examples/bds_mechanism_probe.rs`). Ticks = ms / 50. Watcher
timings are the reliable ones: `Probe::during` only polls the watcher, so actor packets that arrive during an
action are printed in one burst when it ends (same ms). Block hashes resolved by FNV-1a over `{name, states}` NBT.
Packet dumps are cut at 600 chars: AddEntity metadata after `Variant` and most UpdateSubchunkBlocks are truncated.

General wire facts seen everywhere:
- Several changes in one subchunk in one tick go out as one `UpdateSubchunkBlocks` (flags 3, transition 0,
  entity id raw `u64::MAX`); a lone change is an `UpdateBlock` (flags 3). sapling always sends one UpdateBlock per
  cell (send.rs:50-52). Equivalent for clients; low priority.
- `UpdateBlockSynced.entity_unique_id` is written by BDS as an unsigned varint64 of the id's bits; acacia-proto reads
  it with `read_zigzag64` (`acacia-proto/src/generated/packets/part_04.rs:984`), hence the odd values
  (`9223372034707292164` = zigzag-decode of `-4294967288`). Fix the schema before sapling emits this packet.

## Falling blocks

### BDS does
- Support broken: watcher gets `UpdateBlock(air)` on the support at T, then at T+1 (39-73 ms in 6 drills):
  `UpdateBlockSynced{pos = falling cell, air, layer 0, transition Create, entity_unique_id = falling entity}` →
  `AddEntity` → `SetEntityData(Flags)` → `MoveEntityDelta`, all in one batch, same for actor and watcher.
- Placed by hand into air (sand beside stone): `UpdateBlock(sand)` + `place` sound + `UpdateBlockSynced(air, Create)`
  + `AddEntity` all in the same batch (watcher 98249 ms): **0 ticks**, not 2.
- AddEntity: `minecraft:falling_block`, unique id negative (`-4294967288`), position `(x+0.5, y+0.46, z+0.5)`,
  velocity `(0, -0.0392, 0)` (one actor tick already applied: spawn at `y+0.5`, `vy -= 0.04`, move, `*0.98`).
  Metadata starts `Flags = HAS_COLLISION|HAS_GRAVITY (bits 48,49)`, `Health Int 1`, `Variant Int = block network
  hash` (sand 138639715, gravel 1529044762, red_concrete_powder 409760431); the rest (bounding box) is truncated.
- Same batch: `SetEntityData Flags |= MOVING (bit 34)` and `MoveEntityDelta y = y+0.3808` (second tick).
- In flight: `MoveEntityDelta` every tick, `x/z None`, `y Some`, `on_ground false`, `ticks 0`, to both bots.
  `SetEntityMotion` only now and then: `vy -0.22374894` 4-5 ticks after spawn, `-0.4527161` ~12 ticks after spawn
  (4-block drop), then `0` on landing. Not every tick.
- Wire position = AABB bottom + 0.49 (height 0.98): landed y is `top_of_support + 0.49` (-59.51 on dirt at -60,
  -59.01 on a bottom slab, -60.51 in the water cell at -61, -58.51/-57.51 in the stack).
- Landing tick L: `MoveEntityDelta(y, on_ground true)` + `SetEntityMotion(0)`; anvil adds
  `LevelEvent SoundAnvilFall` at the integer cell corner `(-30, -60, 2)`, data 0, on L, before the block.
  L+1 (sand #1: same batch as L): `UpdateBlockSynced{landing cell, block, transition Destroy, same entity id}`.
  L+2: `SetEntityData Flags` back to `HAS_COLLISION|HAS_GRAVITY`. `RemoveEntity` at L+4 (180-209 ms), i.e. ~3 ticks
  after the block. Same packets to actor and watcher.
- Fall time sand from y-57 to dirt top -60: 12 ticks (AddEntity → on_ground delta 616 ms).
- Refused landing (gravel onto torch, sand onto bottom slab): no on_ground delta, no landing motion. On the landing
  tick `LevelEvent ParticleDestroy` at the entity position (`(x+.5, top+0.49, z+.5)`, data = block hash), then
  0-1 tick later `AddItemEntity` (the block's own item, at the entity, with motion) and `RemoveEntity` (actor saw
  Add then Remove, watcher Remove then Add). Torch and slab untouched (no UpdateBlock).
- Anvil lands as `minecraft:anvil[cardinal_direction=south]` (undamaged). Snow layer falls and lands as
  `snow_layer[height=0]`.
- Concrete powder into a 1-deep water pit: `LevelSoundEvent "splash"` at the entity on entering water
  (`entity_type minecraft:falling_block`, entity id set, extra_data 1653280), lands on the floor under the water
  (y -60.51), `UpdateBlockSynced` puts `red_concrete_powder` into the water cell. No hardening (no water left
  beside it).
- Stack of sand/sand/gravel: bottom first, the next one each tick (91772, 91835, 91882 ms); landings and blocks
  also one tick apart.

### sapling differs
- falling.rs:98 + 103: start is `send::set` (UpdateBlock) then AddEntity. BDS uses `UpdateBlockSynced`
  (transition Create, entity unique id) and the land is `UpdateBlockSynced` Destroy (falling.rs:150 via
  `send::set`). Fix: add a synced variant of `send::set` taking the entity id and transition.
- falling.rs:26 `DELAY = 2` for a placed block: BDS falls in the placement tick. Fix: due `now` (0) for the
  self-change in updates.rs:90 when the cell was just placed; keep 1 for "below changed".
- falling.rs:101-102 + 36: spawn at bottom+0.5 with velocity 0; first move a tick later. BDS: AddEntity already at
  `+0.46`, `vy -0.0392`, plus a same-tick delta. Fix: run one `fall` step before announcing (or announce with
  `y+0.46`, `-0.0392`).
- falling.rs:36 `WIRE_OFFSET 0.5`: BDS is bottom+0.49 (spawn bottom = cell+0.01). Landed y is off by 0.01.
- falling.rs:205-206: `SetEntityMotion` every tick. BDS sends it rarely (see above) and always on landing. Fix: send
  motion only on landing (and optionally when |Δv| since last sent is large); keep the per-tick delta.
- falling.rs:174, :205: `ticks: tick` / `tick` field = server tick. BDS sends 0 in both for this entity.
- falling.rs:209 + 33: block placed on the landing tick, entity removed 3 ticks after landing. BDS: block at L+1,
  removal at L+4. Fix: land on the tick after the on_ground move; keep LINGER 3 from the placement.
- falling.rs:199, 206, 151: a refused landing still sends the on_ground delta + motion, keeps the entity for
  LINGER ticks, and puts the particles at the cell centre. BDS: no landing packets, particles at the entity's wire
  position, item + RemoveEntity right away. Fix: in `fall`, when `land` fails, `mobs::remove` now and pass the
  wire position to `break_up` (falling.rs:141).
- falling.rs:155: anvil sound at `centre(cell)`; BDS at the integer corner, on the landing tick before the block.
- sapling-data entity.rs:88-99/119-142: `plain()` metadata has no `Health` (BDS sends Int 1) and flags never get
  MOVING (BDS sets bit 34 while airborne, clears it at L+2). Low priority; needs the untruncated AddEntity to
  confirm the bounding box (sapling sends 0.98 × 0.98).
- No `splash` LevelSoundEvent when a falling block enters water (low).

### Surprises vs falling-blocks.md
- "onPlace queues a tick, delay 2": a hand-placed sand fell in the same tick.
- "spawn position (x+.5, y+.5, z+.5), velocity 0": the announced state is one tick in (`+0.46`, `-0.0392`).
- "expect a SetActorMotion and a MoveActorDelta roughly every tick": delta yes, motion no.
- "UpdateBlock for the placed block comes on the landing tick … RemoveActor 3 ticks later": the block is
  UpdateBlockSynced, one tick after the on-ground delta; removal 3 ticks after the block.
- "For a refused landing, on the same tick: AddItemEntity, LevelEvent 2001, RemoveActor": particles first, item and
  removal up to a tick later.

### Drill bugs
- Concrete powder fell into a lone 1-deep pit: it displaced the water, so hardening was never tested. Put water
  beside the landing cell (or a 2-deep pool for mid-flight hardening).
- AddEntity metadata cut at 600 chars: bounding box and the rest unseen. Raise the cut for AddEntity.

## Pressure plates

### BDS does
- Nothing. No `UpdateBlock`, `LevelSoundEvent` or batch touching any plate cell in all 7 cases: player tp'd onto
  stone, light weighted and wooden plates; spectator on stone; chicken summoned onto a wooden plate (it walked
  z 3.5 → 5.2 across it over ~1 s); armor stand on stone and pig on heavy weighted (neither moved, 3 s each). The
  bot's own view read `redstone_signal=0` throughout, so the plates did exist server-side.

### sapling differs
- Not testable from this capture. Unverified sapling choices still open: stored `redstone_signal=1` for a pressed
  plain plate (plates.rs:61-66, from the decompile only), and mobs never press (plates.rs:114-123).

### Drill bugs
- Not only a teleport problem: even a summoned chicken walking across a wooden plate pressed nothing, so the
  usual "a teleported player skips entityInside" explanation does not cover it. Unexplained; candidates: the
  undecoded `entityInside` early return (pressure-plates.md "Unverified", `actor+0x210 bit0`), or plates placed by
  `/setblock` lacking something a hand-placed plate has. Rerun with: a hand-placed plate, the bot walking onto it
  with movement input (not tp), a dropped item on a wooden plate, a lamp beside it, and `/testforblock` after
  each step to read the server state rather than the bot's.
- The armor stand and the pig never moved, so if BDS only checks plates on movement they could not press.

## Buckets

### BDS does
- Sounds: `LevelSoundEvent` `bucket.fill.water`, `bucket.empty.water`, `bucket.empty.lava`, `bucket.fill.lava`, at
  the **player's feet** (`(0.5, -60, 0.5)` while the clicks were 2 blocks away; `(-0.315, -60, 1.824)` after the
  player was pushed by water), `extra_data -1`, `entity_type ""`, `entity_unique_id -1`. Same tick as the block
  change, after it. Watcher and actor alike.
- Fill water source in a pit: `UpdateBlock(air, layer 0)` + sound. Fill lava source: same with `bucket.fill.lava`.
- Empty bucket on flowing water (depth 3): nothing changes, no sound.
- Water bucket on oak stairs: `UpdateBlock(water depth 0, layer 1)` only, stairs untouched; water then spreads out
  of the stairs (first neighbour ~6 ticks later, then 5 ticks a step). Empty bucket on the wet stairs:
  `UpdateBlock(air, layer 1)` + `bucket.fill.water`.
- Lava bucket into a pit: `UpdateBlock(lava depth 0, layer 0)` + `bucket.empty.lava`.
- Water bucket on a snow layer (height 0): `LevelEvent ParticleDestroy` at the integer corner `(1.0, -60.0, -2.0)`,
  data = snow hash; then one `UpdateSubchunkBlocks` with air then water source at the snow cell (layer 0); sound.
- Water bucket into short grass: `UpdateBlock(water source, layer 0)` + `AddItemEntity` (seeds, network id 291) +
  sound; no ParticleDestroy for the grass.
- Survival items (InventoryContent slot 0/1): bucket ×2 on a source → slot0 bucket ×1, slot1 water_bucket ×1;
  water_bucket → bucket in place; bucket → water_bucket / lava_bucket in place. Ids: 363 bucket, 365 water_bucket,
  366 lava_bucket.
- Adventure lava bucket: rejected; nothing placed, no sound, no inventory packets.
- Every click (success or not) sent the actor only: 6 `UpdateBlock` layer 0 — clicked, clicked+up, clicked-down,
  target, target+up, target-down. Successes also got InventoryContent (Inventory, Armor, Ui, Offhand) before and
  after them. Likely BDS correcting the bot's legacy transaction; a vanilla client may not see this.

### sapling differs
- buckets.rs:112 → send.rs:60-61: snow `ParticleDestroy` at the cell centre; BDS at the integer corner.
- buckets.rs:50 resync only on failure, with both layers (send.rs:174-191), positions clicked/target/target-down;
  BDS resyncs layer 0 of clicked±y and target±y. Low priority, and possibly bot-induced.
- Otherwise matches: sounds at feet with -1 (buckets.rs:140-144), flowing water not taken (buckets.rs:64), layer-1
  pour/take for stairs (liquid.rs:177-200), grass loot (buckets.rs:120-123), item swaps (buckets.rs:148-162),
  adventure refused (buckets.rs:58-60, assuming a not-`allowed` event is refused by default).

### Surprises vs buckets-lava.md
- Snow particles: the note says at `t.down`; confirmed as the integer corner, not `+0.5`.
- None else; sound placement and ids match.

### Drill bugs
- "lava bucket aimed into the ground" (4.66 blocks) and "creative, a water bucket" (5.23) were refused by the bot's
  own reach check: never sent. The player had been washed off its spot by the water poured earlier (see sound
  positions), so later targets drifted out of reach. Stand again (`stand`) before each click, or contain the water.
- "again, on what is left" clicked air (the source was already taken): tests nothing about the second bucket.
- The stairs water ran into the emptied pit at (0,-61,2) and across the drill area, mixing later steps.

## Lava

### BDS does
- East wall of a lava source broken (T): `UpdateBlock lava depth 2` in the gap at T+30 ticks (1517 ms), then
  depth 4 on three cells at +30, depth 6 at +30 (all in UpdateSubchunkBlocks). Drop-off 2, 30 ticks a step.
- Water reaches flowing lava (depth 4): in one batch the water cell (depth 1) and `cobblestone` at the lava cell;
  `LevelEvent SoundFizz` at the lava cell centre `(x+.5, y+.5, z+.5)` in the batch just before it.
  Fizz `data` is random: 1972, 2136, 2356, 2361, 2408, 2417, 2435, 2483, 2764, 2962, 3001, 3023, 3283
  (pitch ×1000, 2.6 ± 0.8).
- Water: 5 ticks a step, drop-off 1, recedes one depth per step.
- Lava column (neighbour `/setblock` at T): falling lava (`depth 8`) one cell down at T+30; spreads sideways at the
  top at +30; lava falling into **flowing** water at y-60 turns that water cell to `stone` with a fizz there.
- No obsidian anywhere in the capture.

### sapling differs
- harden.rs:21, 57: fizz data constant 2600; BDS random `((r - r) * 0.8 + 2.6) * 1000`.
- Delays, drop-off, lava→stone on water below (flow.rs:137-141), flowing lava→cobblestone (harden.rs:35-36): match.

### Surprises vs water.md
- None in what was seen. Obsidian (source + water) was not reached.

### Drill bugs
- The column drill was flooded by the previous step's water (water depth 5-7 reached (16..18,-60,-1..-4)), so the
  lava fell onto flowing water at y-60, never onto the intended source pit at (18,-61,-3). Place the column further
  away or wait for the water to recede.
- Most lava/water batches are cut at 600 chars; the lava source's own fate (obsidian?) is not visible.

## Waterlogging

### BDS does
- Stairs placed into a water source: `UpdateBlock(stairs, layer 0)` + `place` sound only. **No layer-1 packet**; the
  server did keep the water (breaking the stairs later left a source).
- Breaking the wet stairs: `ParticleDestroy` (stairs hash) then `UpdateBlock(water depth 0, layer 0)`. **No layer-1
  air packet.**
- Stone into a source: `UpdateBlock(stone, layer 0)` + sound; nothing on layer 1.
- Stairs into flowing water (depth 1, next to the source): `UpdateBlock(stairs)` + sound; the water is not kept —
  the downstream cells, now unfed, go 2 → 4 → 5 → 6 → 7 → air one step per 5 ticks.
- Bucket into stairs (above): layer-1 `UpdateBlock` both ways — explicit `setExtraBlock` writes are sent.
- `place` sound `extra_data` is the item's default block state hash (`oak_stairs[weirdo_direction=0]`,
  -122484237), not the placed state (direction 2 / 1).

### sapling differs
- send.rs:133-135: `set` also sends a layer-1 UpdateBlock whenever `waterlog::written` moves water between layers
  (stairs into a source → layer-1 water; breaking wet stairs → layer-1 air). BDS sends only layer 0 and lets the
  client apply the same `LevelChunk::setBlock` rule. Fix: drop the implicit layer-1 packet in `set`; keep
  `set_liquid`'s explicit one (buckets, flowing water into Noreaction blocks).
- place.rs:118: place sound uses the placed state's hash; BDS uses the item's block (default state). Low.
- waterlog.rs:41: flowing water not kept on placement — matches.

### Surprises vs waterlogging.md
- The note says nothing about networking; the implicit layer-1 move is not broadcast.

### Drill bugs
- The stream was set up inside `setup` (silenced), so how flowing water meets stairs and grass was never captured;
  only the end state (grass → water depth 3, stairs still stairs). `Probe::name` reads layer 0 only, so whether
  the stairs took flowing water into layer 1 is unknown. Start the stream with `command`, and read layer 1.
- "the wet stairs" break is logged `Err(Rejected("server restored oak_stairs"))`, yet BDS broke it (particles, then
  water source) 3.1 s after the first punch: the bot's break check misjudged a slow survival-speed break.
