# Bedrock scheduled block ticks: storage, order, randomness (BDS, bdsre)

Read 2026-10-07 from `acacia/.testserver/bds-blocks/bedrock_server.exe` (byte-identical to `bds-molang`, PE stamp
6ab54bee) with `py -3 bdsre.py c|dis <va>`, layouts from LeviLamina `BlockTickingQueue.h`, `TickNextTickData.h`,
`MovePriorityQueue.h`, `LevelChunk.h`, `ChunkTickOffsetManager.h`. No `name` labels added; suggested labels below.

| VA | Suggested label | What |
|---|---|---|
| 1415f50f0 | `BlockTickingQueue::add` | load check, then push (or tick now if delay < 0) |
| 141603c30 | `BlockTickingQueue::_addToNextTickQueue` | `push_back` + `push_heap` into `mNextTickQueue` |
| 141603fd0 | `BlockTickingQueue::addTickToLevelChunk` | same push, into `chunk->mTickQueue` |
| 1416042f0 | `BlockTickingQueue::tickPendingTicks` | `(region, until, max, instaTick)` |
| 141617160 | `std::pop_heap<BlockTick, greater>` | MSVC `_Pop_heap_unchecked` |
| 141604c50 | `TickDataSet::_pruneQueueForMemory` | capacity shrink only, no reorder |
| 1409f98a0 | `LevelChunk::tickImpl` | random ticks, then both tick queues, max 100 each |
| 142fe94a0, 146629910 | chunk tick callers | 146629910: per ticking actor, its offset list |
| 142ce9e00 | `Block::tick` (dispatch) | called with the thread-local `Random` |
| 1411889d0 | `Random::Random()` | MT19937 seeded from 4 bytes of 141ccbe30 |

## Storage (verified)

Entry = `BlockTick`, 0x28 bytes: `+0 mIsRemoved`, `+8 pos (x,y,z)`, `+0x14 priorityOffset`, `+0x18 Block*` (exact
state), `+0x20 tick = queue.mCurrentTick + delay` (u64). Queue = `LevelChunk+0x130` (`mTickQueue`; `+0x138` is
`mRandomTickQueue`), so **one queue per chunk**. Callers take `BlockSource+0xe0` instead when set (override queue,
not in normal play), else the chunk at `pos >> 4` (`BlockSource` vt+0x148).

```
add(queue, region, pos, block, delay, prio):                 # 1415f50f0
    if any chunk within ±8 blocks (x, z) of pos is not loaded (region vt+0x70):
        addTickToLevelChunk(chunk_at(pos), ...); return       # 141603fd0: identical push
    if delay < 0: block.tick(region, pos, tls_random); return # immediate
    _addToNextTickQueue(pos, block, delay, prio)              # 141603c30

_addToNextTickQueue:
    next.push_back({removed=false, pos, prio, block, tick=current+delay})
    std::push_heap(next, greater)                             # sift up while parent > new (strict)
    mark chunk dirty
```

- **No dedup in `add`**: a second tick for the same position is a second entry; both run. Liquids pass `prio 0`
  everywhere (148b5d060, 148b5e580, 148b5cd30, 148b617c0 all `add(.., delay, 0)`).
- The only dedup is the caller's: `LiquidBlock::neighborChanged` (148b5cd30) scans `next` linearly and skips the add
  when an entry exists with `!removed && block.type == this && tick == current + delay && pos == pos` (exact due
  tick only). `_tick` and `_trySpreadTo` add unconditionally.
- `remove(pos, block)` (1415f5520) only sets `mIsRemoved`; the entry stays in the heap until it reaches the top.

## Order (verified)

```
tickPendingTicks(q, region, until, max, insta):               # 1416042f0
    if until != -1 and q.current == until: return false        # once per game tick
    n = min(max, next.size); q.current = until; moved = 0
    while next nonempty and moved < n:                         # removed entries do not count
        top = next.front
        if top.removed: pop_heap(next); continue
        if top.tick > until: break
        pop_heap(next); active.push_back(top); push_heap(active); moved++
    prune(next)
    while active nonempty:                                     # insta=false path
        e = active.front
        if any chunk within ±8 of e.pos unloaded:
            if chunk(e.pos) loaded: _addToNextTickQueue(e.pos, e.block, 0, e.prio)   # retried next tick
        else:
            b0 = region.getBlock(e.pos); b1 = region.getExtraBlock(e.pos)
            if b0.type == e.block.type and b0 != air: Block::tick(b0, region, e.pos, tls_random)
            if b1.type == e.block.type and b1 != air: Block::tick(e.block, ...)       # layer 1
        pop_heap(active)
```

- Container: `MovePriorityQueue` = `std::vector` + MSVC `push_heap`/`pop_heap`, comparator `greater` on
  **(tick, priorityOffset) only** (141617160: `a.tick > b.tick || (== && a.prio > b.prio)`). It is a binary
  min-heap: **ties do not keep insertion order**, and there is no third key. Two heaps are used per tick (`next` ->
  `active`), each with MSVC's hole-to-bottom pop (on equal children it takes the right one).
- The order among equal keys is a deterministic function of the whole heap history of that chunk (every entry
  pushed/popped, other due ticks, removed entries). A Python port of 141603c30 + 141617160 + the loop above
  (not kept) gives: k equal entries alone come out in insertion order for k <= 4, but 5 give
  `e0,e4,e3,e2,e1`; two ties A-then-B come out B first in 36% of random histories with unrelated other entries.
- Ticks queued while `active` runs go to `next` (even delay 0), so they run on a later game tick at the earliest.
- Limit: `max = 100` per chunk queue per game tick (1409f98a0 passes 100); the rest stay in `next`.
- The ticked block is the one now at `pos` if its `BlockType` matches the entry's; a state change of the same block
  type (lava depth 8 -> 1) does not cancel the tick.

## Chunks (verified unless marked)

- `LevelChunk::tickImpl` (1409f98a0): random block ticks first, then `tickPendingTicks(mTickQueue, now, 100, 0)`,
  then `mRandomTickQueue`. A chunk ticks once per game tick (`chunk+0x128 mLastTick < now`, 146629910/142fe94a0).
- Chunk order (146629910): for each actor with `ActorTickNeededComponent`, the chunks at `actor chunk + offset` for
  each offset in `Level::getTickingOffsets()` (vt+0x828; `getClientTickingOffsets` vt+0x830 for a local player),
  skipping chunks already ticked this tick. **Inferred**: `ChunkTickOffsetManager::shuffleTickingOffsets(Random&)`
  (header) shuffles that list, so cross-chunk order is randomised; its caller and Random were not found.

## Randomness

- Block ticks get the thread-local `Random` (TLS +0xe60, built by 1411889d0 on first use): MT19937 seeded from 4
  bytes returned by 141ccbe30 (a virtual byte source; **inferred** OS randomness). Not the world seed: differs per
  run. Lava's `rand(4) != 0 -> delay * 4` (water.md) draws from it.
- Liquid delay and flow have no other random input (water.md pseudo-code; `_tick` 148b5d060 calls the Random only
  for the lava recede).
- Run-dependent heap history, beyond lava rolls: the ±8-block load check (re-queue with delay 0), harness commands
  landing in different game ticks, other blocks' ticks in the same chunk (fire uses random delays).

## Consequences for the lava drill (inferred)

- Spread (from the depth-8 cell P) and fall (from the depth-10 cell above C) are both `prio 0` and due on the same
  tick; whichever `tick` runs first fills C and the other's `spread_to` returns (`liq(C)` is lava). If P and C are
  in one chunk the winner is set by heap shape, not insertion order: usually one way, flipped by any difference in
  that chunk's queue history (a lava recede roll is enough, since it moves an entry's due tick by x4).
- Sapling (`liquid.rs` `Flows`): BTreeMap by (due, insertion counter), global, with a `queued` set that drops a
  second schedule for a cell already queued. BDS: per-chunk heaps, ties by heap shape, duplicates kept and both run,
  100 per chunk per tick.
